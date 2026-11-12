//! PR 8 weather: presets, Beer-Lambert storm bases, sun dimming, lightning.

use super::Environment;
use super::sky::{SkyMode, SkySettings};
use super::weather::{Precip, Weather, WeatherPreset};
use bevy::math::Vec3;

fn overcast(thickness: f32) -> SkySettings {
    let mut env = Environment::default();
    WeatherPreset::Overcast.apply(&mut env);
    env.sky.time_of_day = 12.0;
    env.sky.weather.thickness = thickness;
    env.sky
}

/// Mean radiance looking up over a patch of sky.
fn up_mean(s: &SkySettings) -> f32 {
    let mut sum = 0.0;
    for i in 0..64 {
        let a = i as f32 * 0.37;
        sum += s.radiance(Vec3::new(a.cos() * 0.4, a.sin() * 0.4, 1.0)).length();
    }
    sum / 64.0
}

#[test]
fn every_preset_applies_and_sets_the_earth_sky() {
    for p in WeatherPreset::ALL {
        let mut env = Environment::default();
        p.apply(&mut env);
        assert_eq!(env.sky.mode, SkyMode::Earth, "{}", p.label());
        assert_eq!(env.sky.weather.preset, p);
    }
    let mut env = Environment::default();
    WeatherPreset::Thunderstorm.apply(&mut env);
    let w = env.sky.weather;
    assert!(w.lightning && w.precip == Precip::Rain && w.thickness >= 0.9 && env.sky.clouds.coverage >= 0.95);
    WeatherPreset::Fog.apply(&mut env);
    assert!(env.fog && env.fog_density > 0.05 && !env.sky.weather.lightning);
    WeatherPreset::Clear.apply(&mut env);
    assert!(!env.fog && !env.sky.clouds.enabled && env.sky.weather.precip == Precip::None);
    WeatherPreset::Blizzard.apply(&mut env);
    assert_eq!(env.sky.weather.precip, Precip::Snow);
    WeatherPreset::AshHaze.apply(&mut env);
    assert_eq!(env.sky.weather.precip, Precip::Ash);
}

#[test]
fn thick_clouds_darken_their_bases_by_beer_lambert() {
    let w = Weather { thickness: 0.5, ..Default::default() };
    // T = exp(-tau * depth): monotonic in depth and thickness.
    assert!(w.base_transmittance(1.0) < w.base_transmittance(0.0));
    let thick = Weather { thickness: 1.0, ..w };
    assert!(thick.base_transmittance(0.5) < w.base_transmittance(0.5));
    assert!((Weather { thickness: 0.0, ..w }.base_transmittance(1.0) - 1.0).abs() < 1e-6);
    let (thin, storm) = (up_mean(&overcast(0.0)), up_mean(&overcast(1.0)));
    assert!(storm < thin * 0.5, "storm bases {storm} vs thin {thin}");
}

#[test]
fn overcast_dims_and_softens_the_sun() {
    let clear = SkySettings { mode: SkyMode::Earth, time_of_day: 12.0, ..Default::default() };
    let (_, _, m_clear) = clear.key_light().unwrap();
    let mut storm = overcast(1.0);
    storm.weather.sun_dimming = true;
    let (_, _, m_storm) = storm.key_light().unwrap();
    assert!(m_storm < m_clear * 0.3 && m_storm > 0.0, "{m_storm} vs {m_clear}");
    assert!(storm.weather.sun_softness(&storm) > 0.5);
    storm.weather.sun_dimming = false;
    assert!((storm.key_light().unwrap().2 - m_clear).abs() < 1e-5, "dimming off = clear sun");
    assert_eq!(storm.weather.sun_softness(&storm), 0.0);
}

#[test]
fn lightning_lights_the_storm() {
    let mut s = overcast(1.0);
    let dark = s.clone();
    s.weather.lightning = true;
    let peak = (0..2000)
        .map(|i| {
            let a = i as f32 / 2000.0 * std::f32::consts::TAU;
            let d = Vec3::new(a.cos(), a.sin(), 0.25).normalize();
            (s.radiance(d) - dark.radiance(d)).length()
        })
        .fold(0.0f32, f32::max);
    assert!(peak > 0.5, "flash {peak}");
    assert_ne!(s.key(), dark.key(), "the baked sky refreshes");
}

#[test]
fn old_scenes_load_without_weather() {
    let mut v = serde_json::to_value(SkySettings::default()).unwrap();
    v.as_object_mut().unwrap().remove("weather");
    let s: SkySettings = serde_json::from_value(v).unwrap();
    assert_eq!(s.weather, Weather::default());
}
