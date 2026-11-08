//! WebGL2-Renderer für Gr2-Modelle (Ersatz für die three.js-Szene):
//! Phong-Beleuchtung wie `MeshPhongMaterial` mit `AmbientLight(0xffffff, 0.6)`
//! und `DirectionalLight(0xffffff, 1.2)` bei (200, 300, 200), beidseitige
//! Flächen, Alpha-Test 0.5 für Texturen (Haare, Umhänge, Riemen), sRGB-Ausgabe
//! und das `GridHelper`-Gitter.

use super::orbit::Orbit;
use super::scene::{fit, grid_lines, hex_linear, hex_rgb};
use super::types::Gr2ModelInfo;
use glam::{Mat3, Mat4, Vec3};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    HtmlCanvasElement, HtmlImageElement, WebGl2RenderingContext as GL, WebGlBuffer, WebGlProgram, WebGlShader,
    WebGlTexture, WebGlUniformLocation, WebGlVertexArrayObject,
};

const MESH_VS: &str = r#"#version 300 es
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_normal;
layout(location = 2) in vec2 a_uv;
uniform mat4 u_model;
uniform mat4 u_view_proj;
uniform mat3 u_normal_matrix;
out vec3 v_world;
out vec3 v_normal;
out vec2 v_uv;
void main() {
    vec4 w = u_model * vec4(a_pos, 1.0);
    v_world = w.xyz;
    v_normal = u_normal_matrix * a_normal;
    v_uv = a_uv;
    gl_Position = u_view_proj * w;
}
"#;

const MESH_FS: &str = r#"#version 300 es
precision highp float;
in vec3 v_world;
in vec3 v_normal;
in vec2 v_uv;
uniform sampler2D u_tex;
uniform bool u_has_tex;
uniform vec3 u_color;
uniform vec3 u_light_dir;
uniform vec3 u_cam_pos;
out vec4 o_color;
const float PI = 3.141592653589793;
const vec3 SPECULAR = vec3(0.0056053); // 0x111111 linear
const float SHININESS = 30.0;
vec3 linearToSrgb(vec3 c) {
    return mix(c * 12.92, 1.055 * pow(c, vec3(1.0 / 2.4)) - 0.055, step(vec3(0.0031308), c));
}
void main() {
    vec4 diffuse = vec4(u_color, 1.0);
    if (u_has_tex) {
        diffuse *= texture(u_tex, v_uv);
        if (diffuse.a < 0.5) discard;
    }
    vec3 n = normalize(v_normal);
    if (!gl_FrontFacing) n = -n;
    vec3 v = normalize(u_cam_pos - v_world);
    vec3 l = u_light_dir;
    float dotNL = clamp(dot(n, l), 0.0, 1.0);
    vec3 irradiance = vec3(dotNL * 1.2);
    vec3 lambert = diffuse.rgb / PI;
    vec3 h = normalize(l + v);
    float dotNH = clamp(dot(n, h), 0.0, 1.0);
    float dotVH = clamp(dot(v, h), 0.0, 1.0);
    float fresnel = exp2((-5.55473 * dotVH - 6.98316) * dotVH);
    vec3 F = SPECULAR * (1.0 - fresnel) + vec3(fresnel);
    float D = (1.0 / PI) * (SHININESS * 0.5 + 1.0) * pow(dotNH, SHININESS);
    vec3 specular = irradiance * F * (0.25 * D);
    vec3 color = irradiance * lambert + vec3(0.6) * lambert + specular;
    o_color = vec4(linearToSrgb(color), diffuse.a);
}
"#;

const LINE_VS: &str = r#"#version 300 es
layout(location = 0) in vec3 a_pos;
layout(location = 1) in vec3 a_color;
uniform mat4 u_view_proj;
out vec3 v_color;
void main() {
    v_color = a_color;
    gl_Position = u_view_proj * vec4(a_pos, 1.0);
}
"#;

const LINE_FS: &str = r#"#version 300 es
precision highp float;
in vec3 v_color;
out vec4 o_color;
void main() { o_color = vec4(v_color, 1.0); }
"#;

struct MeshUniforms {
    model: Option<WebGlUniformLocation>,
    view_proj: Option<WebGlUniformLocation>,
    normal_matrix: Option<WebGlUniformLocation>,
    tex: Option<WebGlUniformLocation>,
    has_tex: Option<WebGlUniformLocation>,
    color: Option<WebGlUniformLocation>,
    light_dir: Option<WebGlUniformLocation>,
    cam_pos: Option<WebGlUniformLocation>,
}

struct GpuMesh {
    vao: Option<WebGlVertexArrayObject>,
    buffers: Vec<WebGlBuffer>,
    index_count: i32,
    /// `Some` sobald die Textur hochgeladen ist.
    texture: Option<WebGlTexture>,
    /// Mesh hat eine Textur (wird bis zum Laden nicht gezeichnet, wie
    /// three.js mit einer noch leeren Map).
    textured: bool,
    image: Option<HtmlImageElement>,
    _onload: Option<Closure<dyn FnMut()>>,
}

pub struct Renderer {
    gl: GL,
    canvas: HtmlCanvasElement,
    mesh_prog: Option<WebGlProgram>,
    mesh_u: MeshUniforms,
    line_prog: Option<WebGlProgram>,
    line_view_proj: Option<WebGlUniformLocation>,
    grid_vao: Option<WebGlVertexArrayObject>,
    grid_buf: Option<WebGlBuffer>,
    grid_count: i32,
    meshes: Vec<GpuMesh>,
    model_matrix: Mat4,
    /// Wird bei jedem Modellwechsel erhöht - späte Textur-Ladevorgänge
    /// eines alten Modells werden daran erkannt und verworfen.
    pub generation: u64,
    pub orbit: Orbit,
}

fn compile(gl: &GL, kind: u32, src: &str) -> Result<WebGlShader, String> {
    let s = gl.create_shader(kind).ok_or("Shader konnte nicht erzeugt werden")?;
    gl.shader_source(&s, src);
    gl.compile_shader(&s);
    if gl.get_shader_parameter(&s, GL::COMPILE_STATUS).as_bool().unwrap_or(false) {
        Ok(s)
    } else {
        let log = gl.get_shader_info_log(&s).unwrap_or_default();
        gl.delete_shader(Some(&s));
        Err(log)
    }
}

fn link(gl: &GL, vs: &str, fs: &str) -> Result<WebGlProgram, String> {
    let v = compile(gl, GL::VERTEX_SHADER, vs)?;
    let f = compile(gl, GL::FRAGMENT_SHADER, fs)?;
    let p = gl.create_program().ok_or("Programm konnte nicht erzeugt werden")?;
    gl.attach_shader(&p, &v);
    gl.attach_shader(&p, &f);
    gl.link_program(&p);
    gl.delete_shader(Some(&v));
    gl.delete_shader(Some(&f));
    if gl.get_program_parameter(&p, GL::LINK_STATUS).as_bool().unwrap_or(false) {
        Ok(p)
    } else {
        let log = gl.get_program_info_log(&p).unwrap_or_default();
        gl.delete_program(Some(&p));
        Err(log)
    }
}

fn upload_f32(gl: &GL, target: u32, data: &[f32]) -> Option<WebGlBuffer> {
    let b = gl.create_buffer()?;
    gl.bind_buffer(target, Some(&b));
    let arr = js_sys::Float32Array::from(data);
    gl.buffer_data_with_array_buffer_view(target, &arr, GL::STATIC_DRAW);
    Some(b)
}

fn upload_u32(gl: &GL, target: u32, data: &[u32]) -> Option<WebGlBuffer> {
    let b = gl.create_buffer()?;
    gl.bind_buffer(target, Some(&b));
    let arr = js_sys::Uint32Array::from(data);
    gl.buffer_data_with_array_buffer_view(target, &arr, GL::STATIC_DRAW);
    Some(b)
}

impl Renderer {
    pub fn new(canvas: HtmlCanvasElement) -> Result<Self, String> {
        let opts = js_sys::Object::new();
        let _ = js_sys::Reflect::set(&opts, &"antialias".into(), &true.into());
        let gl: GL = canvas
            .get_context_with_context_options("webgl2", &opts)
            .map_err(crate::tauri::js_err)?
            .ok_or("WebGL2 wird nicht unterstützt")?
            .dyn_into()
            .map_err(|_| "WebGL2 wird nicht unterstützt".to_string())?;

        let mesh_prog = link(&gl, MESH_VS, MESH_FS)?;
        let u = |n: &str| gl.get_uniform_location(&mesh_prog, n);
        let mesh_u = MeshUniforms {
            model: u("u_model"),
            view_proj: u("u_view_proj"),
            normal_matrix: u("u_normal_matrix"),
            tex: u("u_tex"),
            has_tex: u("u_has_tex"),
            color: u("u_color"),
            light_dir: u("u_light_dir"),
            cam_pos: u("u_cam_pos"),
        };
        let line_prog = link(&gl, LINE_VS, LINE_FS)?;
        let line_view_proj = gl.get_uniform_location(&line_prog, "u_view_proj");

        let grid = grid_lines(200.0, 20, 0x444444, 0x2a2a2a);
        let grid_vao = gl.create_vertex_array();
        gl.bind_vertex_array(grid_vao.as_ref());
        let grid_buf = upload_f32(&gl, GL::ARRAY_BUFFER, &grid);
        gl.enable_vertex_attrib_array(0);
        gl.vertex_attrib_pointer_with_i32(0, 3, GL::FLOAT, false, 24, 0);
        gl.enable_vertex_attrib_array(1);
        gl.vertex_attrib_pointer_with_i32(1, 3, GL::FLOAT, false, 24, 12);
        gl.bind_vertex_array(None);

        let aspect = canvas.client_width() as f32 / (canvas.client_height().max(1)) as f32;
        Ok(Self {
            gl,
            canvas,
            mesh_prog: Some(mesh_prog),
            mesh_u,
            line_prog: Some(line_prog),
            line_view_proj,
            grid_vao,
            grid_buf,
            grid_count: (grid.len() / 6) as i32,
            meshes: Vec::new(),
            model_matrix: Mat4::IDENTITY,
            generation: 0,
            orbit: Orbit::new(if aspect.is_finite() && aspect > 0.0 { aspect } else { 1.0 }),
        })
    }

    /// `renderer.setSize(w, h)` + Seitenverhältnis der Kamera.
    pub fn set_size(&mut self, w: u32, h: u32) {
        self.canvas.set_width(w);
        self.canvas.set_height(h);
        let style = self.canvas.style();
        let _ = style.set_property("width", &format!("{w}px"));
        let _ = style.set_property("height", &format!("{h}px"));
        if w > 0 && h > 0 {
            self.orbit.aspect = w as f32 / h as f32;
        }
    }

    fn clear_meshes(&mut self) {
        let gl = &self.gl;
        for m in self.meshes.drain(..) {
            if let Some(img) = &m.image {
                img.set_onload(None);
            }
            for b in &m.buffers {
                gl.delete_buffer(Some(b));
            }
            gl.delete_vertex_array(m.vao.as_ref());
            gl.delete_texture(m.texture.as_ref());
        }
    }

    /// Ersetzt das angezeigte Modell. `on_texture` wird für jede Textur mit
    /// (Generation, Mesh-Index, Bild) aufgerufen, sobald sie geladen ist.
    pub fn set_model(&mut self, model: Option<&Gr2ModelInfo>, on_texture: impl Fn(u64, usize, HtmlImageElement) + Clone + 'static) {
        self.clear_meshes();
        self.generation += 1;
        let Some(model) = model.filter(|m| !m.meshes.is_empty()) else {
            return;
        };
        let gl = self.gl.clone();
        for (idx, mesh) in model.meshes.iter().enumerate() {
            let vao = gl.create_vertex_array();
            gl.bind_vertex_array(vao.as_ref());
            let mut buffers = Vec::new();
            let vertex_count = mesh.vertices.len() / 3;
            if let Some(b) = upload_f32(&gl, GL::ARRAY_BUFFER, &mesh.vertices) {
                gl.enable_vertex_attrib_array(0);
                gl.vertex_attrib_pointer_with_i32(0, 3, GL::FLOAT, false, 0, 0);
                buffers.push(b);
            }
            if mesh.normals.len() >= vertex_count * 3 && vertex_count > 0 {
                if let Some(b) = upload_f32(&gl, GL::ARRAY_BUFFER, &mesh.normals) {
                    gl.enable_vertex_attrib_array(1);
                    gl.vertex_attrib_pointer_with_i32(1, 3, GL::FLOAT, false, 0, 0);
                    buffers.push(b);
                }
            } else {
                gl.disable_vertex_attrib_array(1);
                gl.vertex_attrib3f(1, 0.0, 0.0, 1.0);
            }
            if mesh.uvs.len() >= vertex_count * 2 && vertex_count > 0 {
                if let Some(b) = upload_f32(&gl, GL::ARRAY_BUFFER, &mesh.uvs) {
                    gl.enable_vertex_attrib_array(2);
                    gl.vertex_attrib_pointer_with_i32(2, 2, GL::FLOAT, false, 0, 0);
                    buffers.push(b);
                }
            } else {
                gl.disable_vertex_attrib_array(2);
                gl.vertex_attrib2f(2, 0.0, 0.0);
            }
            // Indizes außerhalb des Vertex-Puffers würden den Draw-Call
            // komplett verwerfen - solche Dreiecke werden ausgelassen.
            let indices: Vec<u32> = if mesh.indices.iter().all(|&i| (i as usize) < vertex_count) {
                mesh.indices.clone()
            } else {
                mesh.indices
                    .chunks_exact(3)
                    .filter(|t| t.iter().all(|&i| (i as usize) < vertex_count))
                    .flatten()
                    .copied()
                    .collect()
            };
            if let Some(b) = upload_u32(&gl, GL::ELEMENT_ARRAY_BUFFER, &indices) {
                buffers.push(b);
            }
            gl.bind_vertex_array(None);

            let mut gpu = GpuMesh {
                vao,
                buffers,
                index_count: indices.len() as i32,
                texture: None,
                textured: mesh.texture.is_some(),
                image: None,
                _onload: None,
            };
            if let Some(src) = &mesh.texture {
                if let Ok(img) = HtmlImageElement::new() {
                    let generation = self.generation;
                    let cb = on_texture.clone();
                    let img2 = img.clone();
                    let onload = Closure::<dyn FnMut()>::new(move || cb(generation, idx, img2.clone()));
                    img.set_onload(Some(onload.as_ref().unchecked_ref()));
                    img.set_src(src);
                    gpu.image = Some(img);
                    gpu._onload = Some(onload);
                }
            }
            self.meshes.push(gpu);
        }

        let f = fit(&model.meshes);
        self.model_matrix = f.model;
        f.apply(&mut self.orbit);
    }

    /// Lädt ein fertig dekodiertes Bild als sRGB-Textur (flipY = false,
    /// Mipmaps, Clamp-to-Edge wie die three.js-Standardwerte).
    pub fn upload_texture(&mut self, generation: u64, idx: usize, img: &HtmlImageElement) {
        if generation != self.generation {
            return;
        }
        let gl = &self.gl;
        let Some(mesh) = self.meshes.get_mut(idx) else { return };
        let Some(tex) = gl.create_texture() else { return };
        gl.bind_texture(GL::TEXTURE_2D, Some(&tex));
        gl.pixel_storei(GL::UNPACK_FLIP_Y_WEBGL, 0);
        gl.pixel_storei(GL::UNPACK_PREMULTIPLY_ALPHA_WEBGL, 0);
        gl.pixel_storei(GL::UNPACK_COLORSPACE_CONVERSION_WEBGL, GL::NONE as i32);
        gl.pixel_storei(GL::UNPACK_ALIGNMENT, 4);
        let ok = gl
            .tex_image_2d_with_u32_and_u32_and_html_image_element(
                GL::TEXTURE_2D,
                0,
                GL::SRGB8_ALPHA8 as i32,
                GL::RGBA,
                GL::UNSIGNED_BYTE,
                img,
            )
            .is_ok();
        if !ok {
            gl.delete_texture(Some(&tex));
            return;
        }
        gl.generate_mipmap(GL::TEXTURE_2D);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MIN_FILTER, GL::LINEAR_MIPMAP_LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_MAG_FILTER, GL::LINEAR as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_S, GL::CLAMP_TO_EDGE as i32);
        gl.tex_parameteri(GL::TEXTURE_2D, GL::TEXTURE_WRAP_T, GL::CLAMP_TO_EDGE as i32);
        gl.bind_texture(GL::TEXTURE_2D, None);
        if let Some(old) = mesh.texture.replace(tex) {
            gl.delete_texture(Some(&old));
        }
        mesh.image = None;
        mesh._onload = None;
    }

    pub fn render(&mut self) {
        self.orbit.update();
        let gl = &self.gl;
        let (w, h) = (self.canvas.width() as i32, self.canvas.height() as i32);
        gl.viewport(0, 0, w, h);
        let bg = hex_rgb(0x1e1e1e);
        gl.clear_color(bg[0], bg[1], bg[2], 1.0);
        gl.clear(GL::COLOR_BUFFER_BIT | GL::DEPTH_BUFFER_BIT);
        gl.enable(GL::DEPTH_TEST);
        gl.depth_func(GL::LEQUAL);
        gl.depth_mask(true);

        let view_proj = self.orbit.projection() * self.orbit.view();
        let vp = view_proj.to_cols_array();

        // Gitter (opak)
        gl.disable(GL::BLEND);
        gl.use_program(self.line_prog.as_ref());
        gl.uniform_matrix4fv_with_f32_array(self.line_view_proj.as_ref(), false, &vp);
        gl.bind_vertex_array(self.grid_vao.as_ref());
        gl.draw_arrays(GL::LINES, 0, self.grid_count);

        if self.meshes.is_empty() {
            gl.bind_vertex_array(None);
            return;
        }

        // Modell
        let u = &self.mesh_u;
        gl.use_program(self.mesh_prog.as_ref());
        gl.uniform_matrix4fv_with_f32_array(u.view_proj.as_ref(), false, &vp);
        gl.uniform_matrix4fv_with_f32_array(u.model.as_ref(), false, &self.model_matrix.to_cols_array());
        let normal_matrix = Mat3::from_mat4(self.model_matrix).inverse().transpose();
        gl.uniform_matrix3fv_with_f32_array(u.normal_matrix.as_ref(), false, &normal_matrix.to_cols_array());
        let light = Vec3::new(200.0, 300.0, 200.0).normalize();
        gl.uniform3f(u.light_dir.as_ref(), light.x, light.y, light.z);
        let cam = self.orbit.position;
        gl.uniform3f(u.cam_pos.as_ref(), cam.x, cam.y, cam.z);
        gl.uniform1i(u.tex.as_ref(), 0);
        gl.active_texture(GL::TEXTURE0);
        // Texturierte Meshes sind in three.js `transparent` -> Alpha-Blending.
        gl.blend_func_separate(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA, GL::ONE, GL::ONE_MINUS_SRC_ALPHA);
        gl.disable(GL::CULL_FACE);

        let white = hex_linear(0xffffff);
        let grey = hex_linear(0xcfcfcf);
        // Wie three.js: erst opake (untexturierte), dann "transparente"
        // (texturierte) Meshes.
        let order = self.meshes.iter().filter(|m| !m.textured).chain(self.meshes.iter().filter(|m| m.textured));
        for m in order {
            if m.index_count == 0 {
                continue;
            }
            if m.textured {
                let Some(tex) = &m.texture else { continue };
                gl.enable(GL::BLEND);
                gl.bind_texture(GL::TEXTURE_2D, Some(tex));
                gl.uniform1i(u.has_tex.as_ref(), 1);
                gl.uniform3f(u.color.as_ref(), white[0], white[1], white[2]);
            } else {
                gl.disable(GL::BLEND);
                gl.bind_texture(GL::TEXTURE_2D, None);
                gl.uniform1i(u.has_tex.as_ref(), 0);
                gl.uniform3f(u.color.as_ref(), grey[0], grey[1], grey[2]);
            }
            gl.bind_vertex_array(m.vao.as_ref());
            gl.draw_elements_with_i32(GL::TRIANGLES, m.index_count, GL::UNSIGNED_INT, 0);
        }
        gl.bind_vertex_array(None);
        gl.bind_texture(GL::TEXTURE_2D, None);
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        self.clear_meshes();
        let gl = &self.gl;
        gl.delete_buffer(self.grid_buf.as_ref());
        gl.delete_vertex_array(self.grid_vao.as_ref());
        gl.delete_program(self.mesh_prog.as_ref());
        gl.delete_program(self.line_prog.as_ref());
        // Browser begrenzen die Zahl gleichzeitiger WebGL-Kontexte - den
        // Kontext beim Abbau sofort freigeben statt auf den GC zu warten.
        if let Ok(Some(ext)) = gl.get_extension("WEBGL_lose_context") {
            // `unchecked_into`: das Erweiterungsobjekt heißt im Browser
            // `WEBGL_lose_context`, ein `instanceof`-Test schlüge fehl.
            ext.unchecked_into::<web_sys::WebglLoseContext>().lose_context();
        }
    }
}
