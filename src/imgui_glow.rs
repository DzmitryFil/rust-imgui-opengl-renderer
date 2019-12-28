use glow::*;
use imgui;
use std::mem;

type GLuint = u32;
type GLint = i32;

pub struct Renderer {
    program: GLuint,
    locs: Locs,
    vbo: GLuint,
    ebo: GLuint,
    font_texture: GLuint,
}

struct Locs {
    texture: GLuint,
    proj_mtx: GLuint,
    position: GLuint,
    uv: GLuint,
    color: GLuint,
}

impl Renderer {
    pub fn new(imgui: &mut imgui::Context, gl: &mut glow::Context) -> Self {
        unsafe {
            let vertex_array = gl
                .create_vertex_array()
                .expect("Cannot create vertex array");
            gl.bind_vertex_array(Some(vertex_array));

            let program = gl.create_program().expect("Cannot create program");

            let (vertex_shader_source, fragment_shader_source) = (
				r#"
					uniform mat4 ProjMtx;
					in vec2 Position;
					in vec2 UV;
					in vec4 Color;
					out vec2 Frag_UV;
					out vec4 Frag_Color;
					
					void main()
					{
						Frag_UV = UV;
						Frag_Color = Color;
						gl_Position = ProjMtx * vec4(Position.xy,0,1);
					}
				"#,
				r#"
					uniform sampler2D Texture;
					in vec2 Frag_UV;
					in vec4 Frag_Color;
					out vec4 Out_Color;

					void main()
					{
						Out_Color = Frag_Color * texture(Texture, Frag_UV.st);
					}
				"#,
            );

            let shader_version = "#version 130";

            let shader_sources = [
                (glow::VERTEX_SHADER, vertex_shader_source),
                (glow::FRAGMENT_SHADER, fragment_shader_source),
            ];

            let mut shaders = Vec::with_capacity(shader_sources.len());

            for (shader_type, shader_source) in shader_sources.iter() {
                let shader = gl
                    .create_shader(*shader_type)
                    .expect("Cannot create shader");
                gl.shader_source(shader, &format!("{}\n{}", shader_version, shader_source));
                gl.compile_shader(shader);
                if !gl.get_shader_compile_status(shader) {
                    panic!(gl.get_shader_info_log(shader));
                }
                gl.attach_shader(program, shader);
                shaders.push(shader);
            }

            gl.link_program(program);
            if !gl.get_program_link_status(program) {
                panic!(gl.get_program_info_log(program));
            }

            for shader in shaders {
                gl.detach_shader(program, shader);
                gl.delete_shader(shader);
            }

            let locs = Locs {
                texture: gl.get_uniform_location(program, "Texture").unwrap(),
                proj_mtx: gl.get_uniform_location(program, "ProjMtx").unwrap(),
                position: gl.get_attrib_location(program, "Position").unwrap(),
                uv: gl.get_attrib_location(program, "UV").unwrap(),
                color: gl.get_attrib_location(program, "Color").unwrap(),
            };

            let vbo = gl.create_buffer().unwrap();
            let ebo = gl.create_buffer().unwrap();

            let current_texture = gl.get_parameter_i32(glow::TEXTURE_BINDING_2D);

            let font_texture = gl.create_texture().unwrap();

            gl.bind_texture(glow::TEXTURE_2D, Some(font_texture));

            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::LINEAR as _,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::LINEAR as _,
            );
            gl.pixel_store_i32(glow::UNPACK_ROW_LENGTH, 0);

            {
                let mut atlas = imgui.fonts();

                let texture = atlas.build_rgba32_texture();
                gl.tex_image_2d(
                    glow::TEXTURE_2D,
                    0,
                    glow::RGBA as _,
                    texture.width as _,
                    texture.height as _,
                    0,
                    glow::RGBA,
                    glow::UNSIGNED_BYTE,
                    Some(&texture.data),
                );

                atlas.tex_id = (font_texture as usize).into();
            }

            gl.bind_texture(glow::TEXTURE_2D, Some(current_texture as _));

            Self {
                program,
                locs,
                vbo,
                ebo,
                font_texture,
            }
        }
    }

    pub fn render<'ui>(&self, gl: &mut glow::Context, ui: imgui::Ui<'ui>) {
		use imgui::{DrawCmd, DrawCmdParams, DrawIdx, DrawVert};

        unsafe {
			gl.active_texture(glow::TEXTURE0);


            gl.enable(glow::BLEND);
            gl.blend_equation(glow::FUNC_ADD);
            gl.blend_func(glow::SRC_ALPHA, glow::ONE_MINUS_SRC_ALPHA);
            gl.disable(glow::CULL_FACE);
            gl.disable(glow::DEPTH_TEST);
            gl.enable(glow::SCISSOR_TEST);
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);

            let [width, height] = ui.io().display_size;
            let [scale_w, scale_h] = ui.io().display_framebuffer_scale;

            let fb_width = width * scale_w;
            let fb_height = height * scale_h;

            gl.viewport(0, 0, fb_width as _, fb_height as _);
            let matrix = [
                2.0 / width as f32, 0.0, 0.0, 0.0,
                0.0, 2.0 / -(height as f32), 0.0, 0.0,
                0.0, 0.0, -1.0, 0.0,
                -1.0, 1.0, 0.0, 1.0,
            ];
            gl.use_program(Some(self.program));
            gl.uniform_1_i32(Some(self.locs.texture), 0);
            gl.uniform_matrix_4_f32_slice(Some(self.locs.proj_mtx), false, &matrix);
            if true /*gl.BindSampler.is_loaded()*/ {
                gl.bind_sampler(0, Some(0));
            }

            let vao = gl.create_vertex_array().unwrap();
            gl.bind_vertex_array(Some(vao));
            gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
            gl.enable_vertex_attrib_array(self.locs.position);
            gl.enable_vertex_attrib_array(self.locs.uv);
            gl.enable_vertex_attrib_array(self.locs.color);
            gl.vertex_attrib_pointer_f32(
                self.locs.position,
                2,
                glow::FLOAT,
                false,
                mem::size_of::<DrawVert>() as _,
                field_offset::<DrawVert, _, _>(|v| &v.pos) as _,
            );
            gl.vertex_attrib_pointer_f32(
                self.locs.uv,
                2,
                glow::FLOAT,
                false,
                mem::size_of::<DrawVert>() as _,
                field_offset::<DrawVert, _, _>(|v| &v.uv) as _,
            );
            gl.vertex_attrib_pointer_f32(
                self.locs.color,
                4,
                glow::UNSIGNED_BYTE,
                true,
                mem::size_of::<DrawVert>() as _,
                field_offset::<DrawVert, _, _>(|v| &v.col) as _,
            );

            let draw_data = ui.render();

            for draw_list in draw_data.draw_lists() {
                let vtx_buffer = draw_list.vtx_buffer();
				let idx_buffer = draw_list.idx_buffer();

                gl.bind_buffer(glow::ARRAY_BUFFER, Some(self.vbo));
                gl.buffer_data_u8_slice(
                    glow::ARRAY_BUFFER,
                    slice_to_byte_slice(vtx_buffer),
                    glow::STREAM_DRAW,
                );

                gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(self.ebo));
                gl.buffer_data_u8_slice(
                    glow::ELEMENT_ARRAY_BUFFER,
                    slice_to_byte_slice(idx_buffer),
                    glow::STREAM_DRAW,
                );

                for cmd in draw_list.commands() {
                    match cmd {
                        DrawCmd::Elements {
                            count,
                            cmd_params:
                                DrawCmdParams {
                                    clip_rect: [x, y, z, w],
                                    texture_id,
                                    idx_offset,
                                    ..
                                },
                        } => {
                            gl.bind_texture(glow::TEXTURE_2D, Some(texture_id.id() as u32));

                            gl.scissor(
                                (x * scale_w) as GLint,
                                (fb_height - w * scale_h) as GLint,
                                ((z - x) * scale_w) as GLint,
                                ((w - y) * scale_h) as GLint,
                            );

                            let idx_size = if mem::size_of::<DrawIdx>() == 2 {
                                glow::UNSIGNED_SHORT
                            } else {
                                glow::UNSIGNED_INT
                            };

                            gl.draw_elements(
                                glow::TRIANGLES,
                                count as _,
                                idx_size,
                                (idx_offset * mem::size_of::<DrawIdx>()) as _,
                            );
                        }
                        DrawCmd::ResetRenderState => {
                            unimplemented!("Haven't implemented DrawCmd::ResetRenderState yet");
                        }
                        DrawCmd::RawCallback { .. } => {
                            unimplemented!("Haven't implemented user callbacks yet");
                        }
                    }
                }
            }
            gl.delete_vertex_array(vao);
        }
	}
}

unsafe fn slice_to_byte_slice<T: Sized>(p: &[T]) -> &[u8] {
    ::std::slice::from_raw_parts(
        (&p[0] as *const T) as *const u8,
        ::std::mem::size_of::<T>() * p.len(),
    )
}

impl Drop for Renderer {
    fn drop(&mut self) {
        // let gl = &self.gl;

        // unsafe {
        //     gl.DeleteBuffers(1, &self.vbo);
        //     gl.DeleteBuffers(1, &self.ebo);

        //     gl.DeleteProgram(self.program);

        //     gl.DeleteTextures(1, &self.font_texture);
        // }
    }
}

fn field_offset<T, U, F: for<'a> FnOnce(&'a T) -> &'a U>(f: F) -> usize {
    unsafe {
        let instance = mem::zeroed::<T>();

        let offset = {
            let field: &U = f(&instance);
            field as *const U as usize - &instance as *const T as usize
        };

        mem::forget(instance);

        offset
    }
}
