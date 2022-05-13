#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WindowMode {
    Windowed,
    BorderlessFullscreen,
}

#[derive(Debug, Clone)]
pub struct WindowSettings {
    pub title: &'static str,
    pub width: u32,
    pub height: u32,
    pub resizable: bool,
    pub cursor_visible: bool,
    pub mode: WindowMode,
}

impl Default for WindowSettings {
    fn default() -> Self {
        WindowSettings {
            title: "app",
            width: 1280,
            height: 720,
            resizable: true,
            cursor_visible: true,
            mode: WindowMode::Windowed,
        }
    }
}

#[derive(Debug)]
pub struct Window {
    raw_window: winit::window::Window,

    pub title: &'static str,
    pub resizable: bool,
    pub cursor_visible: bool,
    pub cursor_icon: winit::window::CursorIcon,
    pub physical_cursor_position: Option<glam::DVec2>,
    pub mode: WindowMode,
}

impl Window {
    pub fn new(
        event_loop: &winit::event_loop::EventLoop<()>,
        window_settings: WindowSettings,
    ) -> Self {
        let mut window_builder = winit::window::WindowBuilder::new()
            .with_title(window_settings.title)
            .with_resizable(window_settings.resizable);

        window_builder = match window_settings.mode {
            WindowMode::Windowed => {
                window_builder
                    .with_inner_size(winit::dpi::LogicalSize::new(window_settings.width, window_settings.height))
            }
            WindowMode::BorderlessFullscreen => {
                window_builder
                    .with_fullscreen(Some(winit::window::Fullscreen::Borderless(event_loop.primary_monitor())))
            }
        };

        let raw_window = window_builder.build(&event_loop).unwrap();

        raw_window.set_cursor_visible(window_settings.cursor_visible);

        Self {
            raw_window,
            title: window_settings.title,
            resizable: window_settings.resizable,
            cursor_icon: winit::window::CursorIcon::Default,
            cursor_visible: window_settings.cursor_visible,
            physical_cursor_position: None,
            mode: window_settings.mode,
        }
    }

    #[inline]
    pub fn request_redraw(&self) {
        self.raw_window.request_redraw();
    }

    pub fn position(&self) -> Option<glam::IVec2> {
        self.raw_window
            .outer_position()
            .ok()
            .map(|position| glam::IVec2::new(position.x, position.y))
    }

    pub fn set_position(&self, position: glam::IVec2) {
        self.raw_window.set_outer_position(winit::dpi::PhysicalPosition {
            x: position.x,
            y: position.y,
        });
    }

    #[inline]
    pub fn set_title(&mut self, title: &'static str) {
        self.title = title;
        self.raw_window.set_title(title);
    }

    #[inline]
    pub fn set_resizable(&mut self, resizable: bool) {
        self.resizable = resizable;
        self.raw_window.set_resizable(resizable);
    }

    #[inline]
    pub fn set_cursor_visibility(&mut self, visible: bool) {
        self.cursor_visible = visible;
        self.raw_window.set_cursor_visible(visible);
    }

    #[inline]
    pub fn set_cursor_icon(&mut self, icon: winit::window::CursorIcon) {
        self.cursor_icon = icon;
        self.raw_window.set_cursor_icon(icon);
    }

    pub fn set_mode(&mut self, mode: WindowMode) {
        self.mode = mode;

        match mode {
            WindowMode::Windowed => {
                self.raw_window.set_fullscreen(None);
            }
            WindowMode::BorderlessFullscreen => {
                self.raw_window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            }
        }
    }

    #[inline]
    pub fn cursor_position(&self) -> Option<glam::Vec2> {
        self.physical_cursor_position
            .map(|p| (p / self.raw_window.scale_factor()).as_vec2())
    }

    pub fn set_cursor_position(&mut self, position: glam::Vec2) {
        let inner_size = self.raw_window.inner_size().to_logical::<f32>(self.raw_window.scale_factor());

        self.raw_window.set_cursor_position(winit::dpi::LogicalPosition::new(
            position.x,
            inner_size.height - position.y,
        ))
        .unwrap_or_else(|e| log::error!("Failed to set cursor position: {}", e));
    }

    #[inline]
    pub fn inner(&self) -> &winit::window::Window {
        &self.raw_window
    }
}
