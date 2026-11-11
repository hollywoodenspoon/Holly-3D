# MiniMax -> Cycles loader, run by:
#   blender --background --factory-startup -E CYCLES --python loader.py -- scene.json
# Rebuilds the exported MiniMax render scene (Z-up, like Blender), sets up
# the Cycles device (HIP + HIP-RT by default), OpenImageDenoise, and writes
# the PNG named in the scene file.
import json
import math
import sys

import bpy
from mathutils import Matrix, Vector

cfg = json.load(open(sys.argv[sys.argv.index("--") + 1]))
scene = bpy.context.scene

# Empty the factory startup scene.
for ob in list(bpy.data.objects):
    bpy.data.objects.remove(ob, do_unlink=True)


def set_input(node, names, value):
    """Sets the first socket that exists (names differ between versions)."""
    for n in names:
        if n in node.inputs:
            node.inputs[n].default_value = value
            return


def rgba(c):
    return (c[0], c[1], c[2], 1.0)


def image_node(nt, path, colorspace):
    """An Image Texture node for a baked map (None when there is no map)."""
    if not path:
        return None
    node = nt.nodes.new("ShaderNodeTexImage")
    node.image = bpy.data.images.load(path)
    node.image.colorspace_settings.name = colorspace
    return node


def link_textures(nt, sh, tex):
    """Wires baked base colour, roughness / metallic and normal maps into the BSDF."""
    base = image_node(nt, tex.get("base"), "sRGB")
    if base:
        nt.links.new(base.outputs["Color"], sh.inputs["Base Color"])
        nt.links.new(base.outputs["Alpha"], sh.inputs["Alpha"])
    orm = image_node(nt, tex.get("orm"), "Non-Color")
    if orm:
        # glTF layout: G = roughness, B = metallic (R = cavity AO, which Cycles traces itself).
        sep = nt.nodes.new("ShaderNodeSeparateColor")
        nt.links.new(orm.outputs["Color"], sep.inputs["Color"])
        nt.links.new(sep.outputs["Green"], sh.inputs["Roughness"])
        nt.links.new(sep.outputs["Blue"], sh.inputs["Metallic"])
    normal = image_node(nt, tex.get("normal"), "Non-Color")
    if normal:
        nm = nt.nodes.new("ShaderNodeNormalMap")
        nt.links.new(normal.outputs["Color"], nm.inputs["Color"])
        nt.links.new(nm.outputs["Normal"], sh.inputs["Normal"])


# --- Materials -------------------------------------------------------------
mats = []
for i, m in enumerate(cfg["materials"]):
    mat = bpy.data.materials.new("%s_%d" % (m["name"], i))
    mat.use_nodes = True
    nt = mat.node_tree
    nt.nodes.clear()
    out = nt.nodes.new("ShaderNodeOutputMaterial")
    if m["unlit"]:
        sh = nt.nodes.new("ShaderNodeEmission")
        sh.inputs["Color"].default_value = rgba(m["base"])
        sh.inputs["Strength"].default_value = 1.0
        nt.links.new(sh.outputs["Emission"], out.inputs["Surface"])
    else:
        sh = nt.nodes.new("ShaderNodeBsdfPrincipled")
        set_input(sh, ["Base Color"], rgba(m["base"]))
        set_input(sh, ["Metallic"], m["metallic"])
        set_input(sh, ["Roughness"], m["roughness"])
        set_input(sh, ["IOR"], m["ior"])
        set_input(sh, ["Transmission Weight", "Transmission"], m["transmission"])
        set_input(sh, ["Alpha"], m["alpha"])
        set_input(sh, ["Emission Color", "Emission"], rgba(m["emission"]))
        set_input(sh, ["Emission Strength"], 1.0)
        nt.links.new(sh.outputs["BSDF"], out.inputs["Surface"])
        try:
            link_textures(nt, sh, m.get("textures") or {})
        except Exception as e:  # a render without maps beats no render
            print("MiniMax: textures skipped:", e)
    mats.append(mat)

# --- Meshes ----------------------------------------------------------------
for k, g in enumerate(cfg["meshes"]):
    p = g["verts"]
    verts = [tuple(p[i:i + 3]) for i in range(0, len(p), 3)]
    faces = [(i, i + 1, i + 2) for i in range(0, len(verts), 3)]
    me = bpy.data.meshes.new("mesh_%d" % k)
    me.from_pydata(verts, [], faces)
    me.update()
    me.polygons.foreach_set("use_smooth", [True] * len(faces))
    n = g["normals"]
    normals = [tuple(n[i:i + 3]) for i in range(0, len(n), 3)]
    try:
        if hasattr(me, "use_auto_smooth"):
            me.use_auto_smooth = True  # Blender < 4.1
        me.normals_split_custom_set(normals)
    except Exception as e:  # custom normals are a nicety
        print("MiniMax: custom normals skipped:", e)
    uvs = g.get("uvs")
    if uvs and len(uvs) == len(faces) * 6:
        me.uv_layers.new(name="UVMap").data.foreach_set("uv", uvs)
    me.materials.append(mats[g["mat"]])
    ob = bpy.data.objects.new("obj%d_%d" % (g["obj"], k), me)
    scene.collection.objects.link(ob)

# --- Lights ----------------------------------------------------------------
for k, l in enumerate(cfg["lights"]):
    ld = bpy.data.lights.new("light_%d" % k, l["type"])
    ld.color = l["color"]
    ld.energy = l["energy"]
    if l["type"] == "SUN":
        ld.angle = min(l["radius"] * 0.05, math.pi)
    else:
        ld.shadow_soft_size = l["radius"]
    if l["spot"]:
        ld.spot_size, ld.spot_blend = l["spot"]
    if hasattr(ld, "cycles"):
        ld.cycles.cast_shadow = l["shadows"]
    ob = bpy.data.objects.new("light_%d" % k, ld)
    ob.location = l["pos"]
    ob.rotation_euler = Vector(l["dir"]).to_track_quat("-Z", "Y").to_euler()
    scene.collection.objects.link(ob)

# --- Camera ----------------------------------------------------------------
c = cfg["camera"]
cd = bpy.data.cameras.new("camera")
if c["ortho"]:
    cd.type = "ORTHO"
    cd.ortho_scale = c["ortho_scale"]
else:
    cd.sensor_fit = "VERTICAL"
    cd.angle_y = c["fov_y"]
cd.clip_start, cd.clip_end = c["clip"]
if c.get("dof"):
    cd.dof.use_dof = True
    cd.dof.focus_distance = c["dof"]["focus"]
    cd.dof.aperture_fstop = c["dof"]["fstop"]
cam = bpy.data.objects.new("camera", cd)
cam.matrix_world = Matrix(c["matrix"])
scene.collection.objects.link(cam)
scene.camera = cam

# --- World -----------------------------------------------------------------
w = cfg["world"]
world = bpy.data.worlds.new("world")
world.use_nodes = True
nt = world.node_tree
bg = nt.nodes["Background"]
if w["hdri"]:
    env = nt.nodes.new("ShaderNodeTexEnvironment")
    env.image = bpy.data.images.load(w["hdri"]["path"])
    env.texture_mapping.rotation[2] = math.radians(-w["hdri"]["rotation"])
    nt.links.new(env.outputs["Color"], bg.inputs["Color"])
    bg.inputs["Strength"].default_value = w["hdri"]["strength"]
else:
    bg.inputs["Color"].default_value = rgba(w["color"])
    bg.inputs["Strength"].default_value = 1.0
scene.world = world

# --- Render settings, device and denoiser -----------------------------------
r = scene.render
r.engine = "CYCLES"
r.resolution_x, r.resolution_y, r.resolution_percentage = cfg["width"], cfg["height"], 100
r.film_transparent = cfg["transparent"]
r.image_settings.file_format = "PNG"
r.image_settings.color_mode = "RGBA"
r.filepath = cfg["output"]
vs = scene.view_settings
view = cfg.get("view") or {"transform": "Standard", "exposure": 0.0, "look": "None"}
try:
    vs.view_transform = view["transform"]
except TypeError:
    vs.view_transform = "Standard"
vs.exposure = view["exposure"]
for look in (view["transform"] + " - " + view["look"], view["look"]):
    try:
        vs.look = look
        break
    except TypeError:
        pass
cy = scene.cycles
cy.samples = cfg["samples"]
cy.max_bounces = cfg["max_bounces"]
cy.device = "CPU"
if cfg["device"] != "CPU":
    try:
        prefs = bpy.context.preferences.addons["cycles"].preferences
        prefs.compute_device_type = cfg["device"]
        prefs.get_devices()
        gpus = [d for d in prefs.devices if d.type == cfg["device"]]
        for d in prefs.devices:
            d.use = d.type == cfg["device"]
        if cfg["device"] == "HIP" and hasattr(prefs, "use_hiprt"):
            prefs.use_hiprt = cfg["hiprt"]
        if gpus:
            cy.device = "GPU"
        else:
            print("MiniMax: no %s device found, rendering on the CPU" % cfg["device"])
    except Exception as e:
        print("MiniMax: GPU setup failed, rendering on the CPU:", e)
cy.use_denoising = False
if cfg["denoise"]:
    # OpenImageDenoise when this Blender build has it (some distro builds
    # don't); otherwise render without denoising rather than fail.
    try:
        cy.denoiser = "OPENIMAGEDENOISE"
        cy.use_denoising = True
        if hasattr(cy, "denoising_use_gpu"):
            cy.denoising_use_gpu = cy.device == "GPU"
    except TypeError:
        print("MiniMax: this Blender has no OpenImageDenoise; denoising off")
print("MiniMax: Cycles on %s, %d samples, denoise=%s" % (cy.device, cy.samples, cfg["denoise"]))
bpy.ops.render.render(write_still=True)
