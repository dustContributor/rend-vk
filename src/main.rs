use std::{collections::HashMap, mem::size_of_val};

use ash::vk;

use glam::{Mat4, Quat, Vec3, Vec4, Vec4Swizzles};
use raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use rend_vk::{
    pipeline::sampler::SamplerKey,
    render_task::TaskKind,
    shader_resource::{
        DirLight, Frustum, Material, MultiResource, ResourceKind, StaticShadow, Transform, ViewRay,
    },
    texture::{MipMap, TextureKind},
    window::WindowContext,
    *,
};
use shader_resource::{Timing, View};

fn create_rgba8_image(name: &str, rgba_pixels: [u32; 4], renderer: &mut renderer::Renderer) -> u32 {
    let quad_texture_mips = [MipMap {
        height: 2,
        width: 2,
        offset: 0,
        index: 0,
        size: 16,
    }];
    let texture_id = renderer.gen_texture(
        name.to_string(),
        format::Format::R8G8B8A8_UNORM,
        TextureKind::T2D,
        &quad_texture_mips,
        quad_texture_mips.iter().map(|e| e.size).sum(),
    );
    let texture = renderer
        .fetch_texture(texture_id)
        .expect("missing texture!");
    unsafe {
        if let Some(b) = &texture.staging {
            std::ptr::copy_nonoverlapping(
                rgba_pixels.as_ptr(),
                b.addr as *mut u32,
                rgba_pixels.len(),
            );
        }
    }
    renderer.queue_texture_for_uploading(texture_id);
    texture_id
}

fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    log_panics::init();
    let window_height = 720.0f32;
    let window_width = 1280.0f32;
    let window_context = WindowContext::new(window_width as u32, window_height as u32);
    let instance_extensions = ash_window::enumerate_required_extensions(
        window_context.window.display_handle().unwrap().as_raw(),
    )
    .unwrap();
    let window_size = window_context.window.inner_size();
    let mut renderer = renderer::make_renderer(
        window_size.width,
        window_size.height,
        true,
        true,
        true,
        instance_extensions,
        |entry, instance, surface| {
            let surface_maybe = unsafe {
                ash_window::create_surface(
                    entry,
                    instance,
                    window_context.window.display_handle().unwrap().as_raw(),
                    window_context.window.window_handle().unwrap().as_raw(),
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
    let fov_y_radians = 75.0f32.to_radians();
    let aspect_ratio = window_width / window_height;
    let near_plane = 0.01f32;
    /*
     * NOT a projection far plane: `proj` is reverse-Z with an infinite far
     * plane, used in shaders for other purposes.
     */
    let far_plane = 256f32;
    let proj = Mat4::perspective_infinite_reverse_rh(fov_y_radians, aspect_ratio, near_plane);
    log::info!("proj {:?}", proj);
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
        mesh.write_vertices(&vertices);
        mesh.write_normals(&normals);
        mesh.write_tex_coords(&tex_coords);
        mesh.write_indices(&indices);
        id
    };

    let gen_frustum = || Frustum {
        far_plane,
        near_plane,
        height: window_height,
        width: window_width,
        inv_height: 1.0 / window_height,
        inv_width: 1.0 / window_width,
        ext_height: window_height,
        ext_width: window_width,
        ext_inv_height: 1.0 / window_height,
        ext_inv_width: 1.0 / window_width,
        fragments_per_meter_plane: 600.0,
        pad0: 0,
    };

    let gen_view_ray = || {
        let do_thing = |v: Vec4| {
            let v = inv_proj * v;
            let v = v / v.w;
            v / -v.z
        };
        /*
         * NDC z = 1.0 is the near plane under the reverse-Z infinite projection.
         * Any NDC z along the same ray yields the same direction (the normalization
         * below cancels it), but 1.0 keeps the value inside the valid [0,1] depth
         * range instead of unprojecting a point behind the camera.
         *
         * Dividing by -z lands on z = -1, i.e. true right-handed view directions in
         * front of the camera, which is what the light shaders expect: they rebuild
         * the view position as ray * near/depth and use it for the shadow lookup.
         */
        let bleft = do_thing(Vec4::new(-1.0, -1.0, 1.0, 1.0));
        let bright = do_thing(Vec4::new(1.0, -1.0, 1.0, 1.0));
        let tright = do_thing(Vec4::new(1.0, 1.0, 1.0, 1.0));
        let tleft = do_thing(Vec4::new(-1.0, 1.0, 1.0, 1.0));
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

    let white_id = create_rgba8_image("white", [0xFFFFFFFFu32; 4], &mut renderer);
    let imagea_id = create_rgba8_image(
        "image_rbbr",
        [0xFF0000FFu32, 0xFFFF0000u32, 0xFF0000FFu32, 0xFFFF0000u32],
        // checkerboard: [0xFF0000FFu32, 0xFFFF0000u32, 0xFFFF0000u32, 0xFF0000FFu32],
        &mut renderer,
    );
    let imageb_id = create_rgba8_image(
        "image_grrg",
        [0xFF00FF00u32, 0xFF0000FFu32, 0xFF00FF00u32, 0xFF0000FFu32],
        // checkerboard: [0xFF00FF00u32, 0xFF0000FFu32, 0xFF0000FFu32, 0xFF00FF00u32],
        &mut renderer,
    );
    let imagec_id = create_rgba8_image(
        "image_bggb",
        [0xFFFF0000u32, 0xFF00FF00u32, 0xFFFF0000u32, 0xFF00FF00u32],
        // checkerboard: [0xFFFF0000u32, 0xFF00FF00u32, 0xFF00FF00u32, 0xFFFF0000u32],
        &mut renderer,
    );
    let black_id = create_rgba8_image("black", [0xFF000000u32; 4], &mut renderer);

    let albedo_ids = [imagea_id, imageb_id, imagec_id];

    // (128, 128, 255) decodes to tangent-space (0, 0, 1), i.e. "unperturbed"
    let normal_id = create_rgba8_image("normal_img", [0xFFFF8080u32; 4], &mut renderer);

    let fullscreen_mesh_id = renderer.gen_mesh(3, 0, 0, 0, 3);
    let quad_mesh_id = gen_quad(&mut renderer);

    let sampler_id = renderer.get_sampler(SamplerKey {
        anisotropy: 1,
        filter: pipeline::file::Filtering::Linear,
        wrap_mode: pipeline::file::WrapMode::ClampToEdge,
        compare_func: pipeline::file::CompareFunc::None,
    });

    let texture_ids = [
        white_id, imagea_id, imageb_id, imagec_id, black_id, normal_id,
    ];
    while texture_ids
        .iter()
        .any(|id| !renderer.is_texture_uploaded(*id))
    {
        std::thread::sleep(std::time::Duration::from_millis(100));
        renderer.render();
    }

    let _ = window_context.event_loop(|| {
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
        let world_dir = Vec3::new(-1.0, 1.0, -1.0).normalize();
        let view_dir = view.transform_vector3(world_dir);
        /*
         * Single-cascade test fit so the shadow path is actually exercised.
         *
         * The cascade projection is REVERSED (near -> 1, far -> 0) to match the
         * engine's reverse-Z convention, but ortho projections need to be finite
         */
        let light_dir = world_dir;
        let scene_center = Vec3::new(0.0, -3.0, 20.0);
        let scene_radius = 32.0f32;
        let light_distance = 60.0f32;
        let light_eye = scene_center + light_dir * light_distance;
        let light_view = Mat4::look_at_rh(light_eye, scene_center, Vec3::Y);
        let cascade_near = 1.0f32;
        let cascade_far = light_distance + scene_radius;
        let light_proj = Mat4::orthographic_rh(
            -scene_radius,
            scene_radius,
            -scene_radius,
            scene_radius,
            cascade_far,
            cascade_near,
        );
        let cascade_view_proj = light_proj * light_view;
        let dir_light = DirLight {
            color: Vec4::new(1.0, 1.0, 1.0, 0.0),
            ground_color: Vec4::new(0.2, 0.0, 0.0, 0.0),
            sky_color: Vec4::new(0.0, 0.0, 0.2, 0.0),
            view_dir: Vec4::new(view_dir.x, view_dir.y, view_dir.z, 0.0),
            cascade_projs: [
                cascade_view_proj,
                cascade_view_proj,
                cascade_view_proj,
                cascade_view_proj,
            ],
            /*
             * Finite ascending upper bounds of each cascade's view-space depth
             * range. There is no camera far plane to use as the terminal split
             * anymore, so the last one is an explicit max shadow distance and
             * everything past it is left unshadowed
             */
            cascade_splits: Vec4::new(15.0, 30.0, 45.0, 60.0),
            cascade_biases: Vec4::splat(0.002),
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

        let quad_resources: HashMap<ResourceKind, MultiResource> = HashMap::new();
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

        const DIR_CASCADES: u32 = 4;
        for kind in [TaskKind::MeshStatic, TaskKind::MeshStaticShadowDir] {
            for (model_mat, albedo_id) in [quad1_model, quad2_model, quad3_model]
                .iter()
                .zip(albedo_ids)
            {
                let mut task_res = quad_resources.clone();
                task_res.insert(
                    ResourceKind::Material,
                    MultiResource::Material(vec![Material {
                        img0: albedo_id,
                        img1: normal_id,
                        img2: black_id,
                        smp0: sampler_id,
                        smp1: sampler_id,
                        smp2: sampler_id,
                        scaling: 1.0,
                        ..Default::default()
                    }]),
                );
                task_res.insert(
                    ResourceKind::Transform,
                    MultiResource::Transform(vec![Transform {
                        model: model_mat.clone(),
                        prev_model: model_mat.clone(),
                    }]),
                );
                let make_task =
                    |resources: HashMap<ResourceKind, MultiResource>| render_task::RenderTask {
                        mesh_buffer_id: quad_mesh_id,
                        instance_count: 1,
                        vertex_count: 6,
                        indices_offset: 0,
                        kind,
                        resources,
                    };
                if kind == TaskKind::MeshStaticShadowDir {
                    for cascade_id in 0..DIR_CASCADES {
                        let mut shadow_res = task_res.clone();
                        shadow_res.insert(
                            ResourceKind::StaticShadow,
                            MultiResource::StaticShadow(vec![StaticShadow {
                                cascade_id,
                                pad0: 0,
                                pad1: 0,
                                pad2: 0,
                            }]),
                        );
                        renderer.add_task_to_queue(make_task(shadow_res), cascade_id);
                    }
                } else {
                    renderer.add_task_to_queue(make_task(task_res), 0);
                }
            }
        }

        renderer.render();
    });
    let mut renderer = renderer;
    renderer.destroy();
}
