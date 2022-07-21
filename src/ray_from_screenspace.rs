use bevy::{
    math::{Mat4, Vec2, Vec3},
    prelude::{Camera, GlobalTransform, Res},
    render::camera::{Projection, RenderTarget},
    window::{Window, Windows},
};

fn get_window_for_camera<'a>(windows: &'a Windows, camera: &Camera) -> Option<&'a Window> {
    match camera.target {
        RenderTarget::Window(window_id) => match windows.get(window_id) {
            None => None,
            window => window,
        },
        _ => None,
    }
}

pub fn ray_from_screenspace(
    cursor_pos_screen: Vec2,
    windows: &Res<Windows>,
    camera: &Camera,
    camera_projection: &Projection,
    camera_transform: &GlobalTransform,
) -> Option<(Vec3, Vec3)> {
    let view = camera_transform.compute_matrix();
    let window = get_window_for_camera(windows, camera)?;
    let screen_size = Vec2::from([window.width() as f32, window.height() as f32]);
    let projection = camera.projection_matrix();

    // 2D Normalized device coordinate cursor position from (-1, -1) to (1, 1)
    let cursor_ndc = (cursor_pos_screen / screen_size) * 2.0 - Vec2::from([1.0, 1.0]);
    let ndc_to_world: Mat4 = view * projection.inverse();
    let world_to_ndc = projection * view;
    let is_orthographic = projection.w_axis[3] == 1.0;

    // Compute the cursor position at the near plane. The bevy camera looks at -Z.
    let camera_near = match camera_projection {
        Projection::Perspective(perspective_projection) => perspective_projection.near,
        Projection::Orthographic(orthographic_projection) => orthographic_projection.near,
    };
    let ndc_near = world_to_ndc.transform_point3(-Vec3::Z * camera_near).z;
    let cursor_pos_near = ndc_to_world.transform_point3(cursor_ndc.extend(ndc_near));

    // Compute the ray's direction depending on the projection used.
    let ray_direction = match is_orthographic {
        true => view.transform_vector3(-Vec3::Z), // All screenspace rays are parallel in ortho
        false => cursor_pos_near - camera_transform.translation, // Direction from camera to cursor
    };

    Some((cursor_pos_near, ray_direction))
}
