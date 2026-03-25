use crate::{Config, GraphicsModeConfig};
use bevy::{
    prelude::{Entity, NonSend, Query, Res, ResMut, With},
    window::{PrimaryWindow, Window, WindowMode},
    winit::WinitWindows,
};
use std::collections::HashSet;

pub fn init_window_system(
    mut config: ResMut<Config>,
    query_window: Query<Entity, With<PrimaryWindow>>,
    winit_windows: NonSend<WinitWindows>,
) {
    let window_entity = query_window.single();
    let window = match winit_windows.get_window(window_entity) {
        Some(window) => window,
        None => {
            log::warn!("Window not found");
            return;
        }
    };

    let monitor = match window.current_monitor() {
        Some(monitor) => monitor,
        None => {
            log::warn!("Monitor not found");
            return;
        }
    };

    let mut resolutions = HashSet::new();
    for mode in monitor.video_modes() {
        let size = mode.size();
        resolutions.insert((size.width, size.height));
    }

    let mut resolutions: Vec<_> = resolutions.into_iter().collect();
    resolutions.sort();

    config.graphics.resolutions = resolutions;
}

pub fn ui_window_system(mut windows: Query<&mut Window>, config: Res<Config>) {
    let mut window = windows.single_mut();

    match config.graphics.mode {
        GraphicsModeConfig::Window { width, height } => {
            window.resolution.set(width, height);
            window.mode = WindowMode::Windowed;
        }
        GraphicsModeConfig::Fullscreen => {
            window.mode = WindowMode::BorderlessFullscreen;
        }
    }
}
