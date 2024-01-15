use std::{collections::HashMap, mem::size_of};

use ash::vk;

use glam::{Mat4, Vec3, Vec4, Vec4Swizzles};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use rend_vk::{
    shader_resource::{Frustum, MultiResource, ResourceKind, Transform, TransformExtra, ViewRay},
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
        Vec3::new(-3.0, 2.0, -3.0),
        Vec3::new(-3.0, 2.0, -2.0),
        Vec3::new(0.0, 1.0, 0.0),
    );
    let fov_y_radians = 60.0f32.to_radians();
    let aspect_ratio = window_width / window_height;
    let near_plane = 0.3f32;
    let far_plane = 256f32;
    let proj = Mat4::perspective_rh(fov_y_radians, aspect_ratio, near_plane, far_plane);
    let inv_proj = proj.inverse();

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
            vertices.len() as u32,
            normals.len() as u32,
            tex_coords.len() as u32,
            indices.len() as u32,
            indices.len() as u32,
        );
        let mesh = renderer.fetch_mesh(id).unwrap();
        unsafe {
            std::ptr::copy_nonoverlapping(
                vertices.as_ptr() as *const u8,
                mesh.vertices.addr as *mut u8,
                size_of::<f32>() * vertices.len(),
            );
            std::ptr::copy_nonoverlapping(
                normals.as_ptr() as *const u8,
                mesh.normals.addr as *mut u8,
                size_of::<f32>() * normals.len(),
            );
            std::ptr::copy_nonoverlapping(
                tex_coords.as_ptr() as *const u8,
                mesh.tex_coords.addr as *mut u8,
                size_of::<f32>() * tex_coords.len(),
            );
            std::ptr::copy_nonoverlapping(
                indices.as_ptr() as *const u8,
                mesh.indices.addr as *mut u8,
                size_of::<u32>() * indices.len(),
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

    let fullscreen_mesh_id = renderer.gen_mesh(3, 0, 0, 0, 3);
    let quad_mesh_id = gen_quad(&mut renderer);
    let quad_model = Mat4::from_translation(Vec3::new(-10.0, 1.0, 10.0));
    // quad_model.add

    window_context.event_loop(|| {
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
        renderer.add_task_to_queue(render_task::RenderTask {
            mesh_buffer_id: quad_mesh_id,
            instance_count: 1,
            kind: render_task::TaskKind::MeshStatic,
            resources: quad_resources,
        });
        renderer.render();
    });
    let mut renderer = renderer;
    renderer.destroy();
}
