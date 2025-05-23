use std::{collections::HashMap, mem::size_of_val};

use ash::vk;

use glam::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use rend_vk::{
    pipeline::sampler::SamplerKey,
    render_task::TaskKind,
    shader_resource::{
        DirLight, Frustum, Material, MultiResource, ResourceKind, StaticShadow, Transform, ViewRay,
    },
    texture::MipMap,
    window::WindowContext,
    *,
};
use shader_resource::{Timing, View};

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
    let view = {
        let pos = Vec3::new(0.0, 0.0, -10.0);
        Mat4::look_at_rh(
            pos,
            pos + Vec3::new(0.0, 0.0, 1.0),
            Vec3::new(0.0, 1.0, 0.0),
        )
    };
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
        let indices = [2u16, 3, 1, 2, 1, 0];
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
                mesh.indices.addr as *mut u16,
                indices.len(),
            );
        };
        id
    };

    let gen_frustum = || Frustum {
        far_plane,
        near_plane,
        height: window_height,
        width: window_width,
        inv_height: 1.0 / window_height,
        inv_width: 1.0 / window_width,
        fragments_per_meter_plane: 600.0,
        pad0: 0,
    };

    let gen_view_ray = || {
        let do_thing = |v: Vec4| {
            let v = inv_proj * v;
            let v = v / v.w;
            v / v.z
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
        height: 2,
        width: 2,
        offset: 0,
        index: 0,
        size: 16,
    }];
    let quad_texture_id = renderer.gen_texture(
        "quad_texture".to_string(),
        format::Format::R8G8B8A8_UNORM,
        &quad_texture_mips,
        quad_texture_mips.iter().map(|e| e.size).sum(),
    );
    let quad_normal_id = renderer.gen_texture(
        "quad_normal".to_string(),
        format::Format::R8G8B8A8_UNORM,
        &quad_texture_mips,
        quad_texture_mips.iter().map(|e| e.size).sum(),
    );
    let quad_albedo = renderer
        .fetch_texture(quad_texture_id)
        .expect("missing texture!");
    let quad_normal = renderer
        .fetch_texture(quad_normal_id)
        .expect("missing texture!");
    unsafe {
        let pixels = [0xFFFFFFFFu32, 0xFFFFFFFFu32, 0xFFFFFFFFu32, 0xFFFFFFFFu32];
        if let Some(b) = &quad_albedo.staging {
            std::ptr::copy_nonoverlapping(pixels.as_ptr(), b.addr as *mut u32, pixels.len());
        }
        let normals = [0x0000FF00u32, 0x0000FF00u32, 0x000000FFu32, 0x000000FFu32];
        if let Some(b) = &quad_normal.staging {
            std::ptr::copy_nonoverlapping(normals.as_ptr(), b.addr as *mut u32, normals.len());
        }
    }
    renderer.queue_texture_for_uploading(quad_texture_id);
    renderer.queue_texture_for_uploading(quad_normal_id);

    let fullscreen_mesh_id = renderer.gen_mesh(3, 0, 0, 0, 3);
    let quad_mesh_id = gen_quad(&mut renderer);

    let sampler_id = renderer.get_sampler(SamplerKey {
        anisotropy: 1,
        filter: pipeline::file::Filtering::Linear,
        wrap_mode: pipeline::file::WrapMode::ClampToEdge,
        compare_func: pipeline::file::CompareFunc::None,
    });

    while !renderer.is_texture_uploaded(quad_texture_id)
        || !renderer.is_texture_uploaded(quad_normal_id)
    {
        std::thread::sleep(std::time::Duration::from_millis(100));
        renderer.render();
    }

    window_context.event_loop(|| {
        renderer.place_shader_resource(
            ResourceKind::Frustum,
            shader_resource::SingleResource::Frustum(gen_frustum()),
        );
        renderer.place_shader_resource(
            ResourceKind::ViewRay,
            shader_resource::SingleResource::ViewRay(gen_view_ray()),
        );
        renderer.place_shader_resource(
            ResourceKind::Timing,
            shader_resource::SingleResource::Timing(Timing {
                interpolation: 0.5,
                pad0: 0,
                pad1: 0,
                pad2: 0,
            }),
        );
        renderer.place_shader_resource(
            ResourceKind::View,
            shader_resource::SingleResource::View(View {
                view,
                proj,
                view_proj: proj * view,
                prev_view: view,
                prev_proj: proj,
                prev_view_proj: proj * view,
                inv_view: view.inverse(),
                prev_inv_view: view.inverse(),
            }),
        );
        let world_dir = Vec3::new(1.0, -1.0, 1.0).normalize();
        let view_dir = view.transform_vector3(world_dir);
        let dir_light = DirLight {
            color: Vec4::new(1.0, 1.0, 1.0, 0.0),
            ground_color: Vec4::new(1.0, 0.0, 0.0, 0.0),
            sky_color: Vec4::new(0.0, 0.0, 1.0, 0.0),
            view_dir: Vec4::new(view_dir.x, view_dir.y, view_dir.z, 0.0),
            cascade_projs: [
                Mat4::IDENTITY,
                Mat4::IDENTITY,
                Mat4::IDENTITY,
                Mat4::IDENTITY,
            ],
            cascade_splits: Vec4::ZERO,
            cascade_biases: Vec4::ZERO,
        };
        renderer.place_shader_resource(
            ResourceKind::DirLight,
            rend_vk::shader_resource::SingleResource::DirLight(dir_light.clone()),
        );
        renderer.add_task_to_queue(
            render_task::RenderTask {
                mesh_buffer_id: fullscreen_mesh_id,
                instance_count: 1,
                vertex_count: 3,
                indices_offset: 0,
                kind: render_task::TaskKind::Fullscreen,
                resources: HashMap::new(),
            },
            0,
        );

        let mut dir_light_res = HashMap::new();
        dir_light_res.insert(
            ResourceKind::DirLight,
            MultiResource::DirLight(vec![dir_light]),
        );
        renderer.add_task_to_queue(
            render_task::RenderTask {
                mesh_buffer_id: fullscreen_mesh_id,
                instance_count: 1,
                vertex_count: 3,
                indices_offset: 0,
                kind: render_task::TaskKind::LightDir,
                resources: dir_light_res,
            },
            0,
        );

        let mut quad_resources: HashMap<ResourceKind, MultiResource> = HashMap::new();
        let quad1_model = Mat4::from_scale_rotation_translation(
            Vec3::new(10.0, 10.0, 10.0),
            Quat::from_euler(
                glam::EulerRot::XYZ,
                -90.0f32.to_radians(),
                0.0,
                45.0f32.to_radians(),
            ),
            Vec3::new(0.0, -2.0, 20.0),
        );
        let quad2_model = Mat4::from_scale_rotation_translation(
            Vec3::new(5.0, 5.0, 5.0),
            Quat::from_euler(
                glam::EulerRot::XYZ,
                -90.0f32.to_radians(),
                0.0,
                -45.0f32.to_radians(),
            ),
            Vec3::new(5.0, -4.0, 17.0),
        );
        let quad3_model = Mat4::from_scale_rotation_translation(
            Vec3::new(50.0, 50.0, 50.0),
            Quat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, 0.0),
            Vec3::new(0.0, -5.0, 20.0),
        );

        quad_resources.insert(
            ResourceKind::StaticShadow,
            MultiResource::StaticShadow(vec![StaticShadow {
                cascade_id: 1,
                pad0: 0,
                pad1: 0,
                pad2: 0,
            }]),
        );
        quad_resources.insert(
            ResourceKind::Material,
            MultiResource::Material(vec![Material {
                diffuse_handle: quad_texture_id,
                normal_handle: quad_normal_id,
                diffuse_sampler: sampler_id,
                normal_sampler: sampler_id,
                scaling: 1.0,
                shininess: 100.0,
                ..Default::default()
            }]),
        );
        for kind in [TaskKind::MeshStatic, TaskKind::MeshStaticShadowDir] {
            for model_mat in [quad1_model, quad2_model, quad3_model] {
                let mut task_res = quad_resources.clone();
                task_res.insert(
                    ResourceKind::Transform,
                    MultiResource::Transform(vec![Transform {
                        model: model_mat,
                        prev_model: model_mat,
                    }]),
                );
                renderer.add_task_to_queue(
                    render_task::RenderTask {
                        mesh_buffer_id: quad_mesh_id,
                        instance_count: 1,
                        vertex_count: 6,
                        indices_offset: 0,
                        kind,
                        resources: task_res,
                    },
                    0,
                );
            }
        }

        renderer.render();
        true
        // renderer.get_current_frame() < 2
    });
    let mut renderer = renderer;
    renderer.destroy();
}
