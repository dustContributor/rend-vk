use std::{
    collections::HashMap,
    mem::{size_of, size_of_val},
};

use ash::vk;

use glam::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use rend_vk::{
    pipeline::sampler::SamplerKey,
    shader_resource::{
        Frustum, Material, MultiResource, ResourceKind, Transform, TransformExtra, ViewRay,
    },
    texture::MipMap,
    window::WindowContext,
    *,
};

fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    log_panics::init();
    let window_height = 720.0f32;
    let window_width = 1280.0f32;
    let window_context = WindowContext::new(window_width as u32, window_height as u32);
    let instance_extensions =
        ash_window::enumerate_required_extensions(window_context.window.raw_display_handle())
            .unwrap();
    let mut renderer = renderer::make_renderer(
        true,
        true,
        true,
        instance_extensions,
        |entry, instance, surface| {
            let surface_maybe = unsafe {
                ash_window::create_surface(
                    entry,
                    instance,
                    window_context.window.raw_display_handle(),
                    window_context.window.raw_window_handle(),
                    None,
                )
            };
            match surface_maybe {
                Err(err) => err,
                Ok(sur) => {
                    unsafe { surface.write(sur) };
                    vk::Result::SUCCESS
                }
            }
        },
    );
    let view = Mat4::look_at_rh(
        Vec3::new(0.0, 1.0, 0.0),
        Vec3::new(0.0, 0.0, -1.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let fov_y_radians = 60.0f32.to_radians();
    let aspect_ratio = window_width / window_height;
    let near_plane = 0.3f32;
    let far_plane = 256f32;
    let proj = Mat4::perspective_rh(fov_y_radians, aspect_ratio, near_plane, far_plane);
    let inv_proj = proj.inverse();

    let quad_model = Mat4::from_scale_rotation_translation(
        Vec3::new(10.0, 10.0, 10.0),
        Quat::from_rotation_y(0.0f32.to_radians()),
        Vec3::new(0.0, -1.0, 0.0),
    );

    let gen_quad = |renderer: &mut renderer::Renderer| {
        let xs = 0.5f32;
        let ys = 0.5f32;
        let vertices = [-xs, 0.0, -ys, xs, 0.0, -ys, -xs, 0.0, ys, xs, 0.0, ys];
        let tex_coords = [0.0f32, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        let normals = [
            0.0f32, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0,
        ];
        let indices = [2u32, 3, 1, 2, 1, 0];
        let id = renderer.gen_mesh(
            size_of_val(&vertices) as u32,
            size_of_val(&normals) as u32,
            size_of_val(&tex_coords) as u32,
            size_of_val(&indices) as u32,
            indices.len() as u32,
        );
        let mesh = renderer.fetch_mesh(id).expect("missing mesh!");
        unsafe {
            std::ptr::copy_nonoverlapping(
                vertices.as_ptr(),
                mesh.vertices.addr as *mut f32,
                vertices.len(),
            );
            std::ptr::copy_nonoverlapping(
                normals.as_ptr(),
                mesh.normals.addr as *mut f32,
                normals.len(),
            );
            std::ptr::copy_nonoverlapping(
                tex_coords.as_ptr(),
                mesh.tex_coords.addr as *mut f32,
                tex_coords.len(),
            );
            std::ptr::copy_nonoverlapping(
                indices.as_ptr(),
                mesh.indices.addr as *mut u32,
                indices.len(),
            );
        };
        return id;
    };

    let gen_frustum = || Frustum {
        far_plane,
        near_plane,
        height: window_height,
        width: window_width,
        inv_height: 1.0 / window_height,
        inv_width: 1.0 / window_width,
    };

    let gen_view_ray = || {
        let do_thing = |v: Vec4| {
            let v = inv_proj * v;
            let v = v / v.w;
            let v = v / v.z;
            return v;
        };
        let bleft = do_thing(Vec4::new(-1.0, -1.0, -1.0, 1.0));
        let bright = do_thing(Vec4::new(1.0, -1.0, -1.0, 1.0));
        let tright = do_thing(Vec4::new(1.0, 1.0, -1.0, 1.0));
        let tleft = do_thing(Vec4::new(-1.0, 1.0, -1.0, 1.0));
        ViewRay {
            m22: proj.z_axis[2],
            m23: proj.z_axis[3],
            m32: proj.w_axis[2],
            m33: proj.w_axis[3],
            bleft: bleft.xyz(),
            bright: bright.xyz(),
            tleft: tleft.xyz(),
            tright: tright.xyz(),
        }
    };

    let quad_texture_mips = [MipMap {
        height: 1,
        width: 1,
        offset: 0,
        index: 0,
        size: 4,
    }];
    let quad_texture_id = renderer.gen_texture(
        "quad_texture".to_string(),
        format::Format::R8G8B8A8_UNORM,
        &quad_texture_mips,
        quad_texture_mips.iter().map(|e| e.size).sum(),
    );
    let quad_texture = renderer
        .fetch_texture(quad_texture_id)
        .expect("missing texture!");
    unsafe {
        let pixel = [0xFF00FF00u32];
        if let Some(b) = &quad_texture.staging {
            std::ptr::copy_nonoverlapping(
                pixel.as_ptr() as *const u8,
                b.addr as *mut u8,
                size_of::<u32>(),
            );
        }
    }
    renderer.queue_texture_for_uploading(quad_texture_id);

    let fullscreen_mesh_id = renderer.gen_mesh(3, 0, 0, 0, 3);
    let quad_mesh_id = gen_quad(&mut renderer);

    let sampler_id = renderer.get_sampler(SamplerKey {
        anisotropy: 1,
        filter: pipeline::file::Filtering::Linear,
        wrap_mode: pipeline::file::WrapMode::ClampToEdge,
    });

    window_context.event_loop(|| {
        renderer.place_shader_resource(
            ResourceKind::Frustum,
            shader_resource::SingleResource::Frustum(gen_frustum()),
        );
        renderer.place_shader_resource(
            ResourceKind::ViewRay,
            shader_resource::SingleResource::ViewRay(gen_view_ray()),
        );
        renderer.add_task_to_queue(render_task::RenderTask {
            mesh_buffer_id: fullscreen_mesh_id,
            instance_count: 1,
            kind: render_task::TaskKind::Fullscreen,
            resources: HashMap::new(),
        });
        if renderer.is_texture_uploaded(quad_texture_id) {
            let mut quad_resources: HashMap<ResourceKind, MultiResource> = HashMap::new();
            quad_resources.insert(
                ResourceKind::Transform,
                MultiResource::Transform(vec![Transform {
                    mv: quad_model * view,
                    mvp: quad_model * view * proj,
                }]),
            );
            quad_resources.insert(
                ResourceKind::TransformExtra,
                MultiResource::TransformExtra(vec![TransformExtra {
                    prev_mvp: quad_model * view * proj,
                }]),
            );
            quad_resources.insert(
                ResourceKind::Material,
                MultiResource::Material(vec![Material {
                    diffuse_handle: quad_texture_id,
                    normal_handle: quad_texture_id,
                    diffuse_sampler: sampler_id,
                    normal_sampler: sampler_id,
                    scaling: 1.0,
                    shininess: 127.0,
                    ..Default::default()
                }]),
            );
            renderer.add_task_to_queue(render_task::RenderTask {
                mesh_buffer_id: quad_mesh_id,
                instance_count: 1,
                kind: render_task::TaskKind::MeshStatic,
                resources: quad_resources,
            });
        }
        renderer.render();
    });
    let mut renderer = renderer;
    renderer.destroy();
}
