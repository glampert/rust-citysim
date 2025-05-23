use std::ffi::c_void;
use glfw::Context;

use crate::{
    utils::{self, Size2D, Point2D, Vec2},
    app::{Application, ApplicationEvent, ApplicationEventList},
};

use super::{
    input::InputSystem
};

// ----------------------------------------------
// These will be exposed as public types in the
// app::input module, so we don't have to
// replicate all the GLFW enums.
// ----------------------------------------------

pub type InputModifiers = glfw::Modifiers;
pub type InputAction = glfw::Action;
pub type InputKey = glfw::Key;
pub type MouseButton = glfw::MouseButton;

// ----------------------------------------------
// GlfwApplication
// ----------------------------------------------

pub struct GlfwApplication {
    title: String,
    window_size: Size2D,
    fullscreen: bool,
    should_quit: bool,
    glfw_instance: glfw::Glfw,
    window: glfw::PWindow,
    event_receiver: glfw::GlfwReceiver<(f64, glfw::WindowEvent)>,
}

impl GlfwApplication {
    pub fn new(title: String, window_size: Size2D, fullscreen: bool) -> Self {
        debug_assert!(window_size.is_valid());

        let mut glfw_instance =
            glfw::init(glfw::fail_on_errors).expect("Failed to initialize GLFW!");

        glfw_instance.window_hint(glfw::WindowHint::ContextVersion(3, 3));
        glfw_instance.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
        glfw_instance.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));

        // TODO: Handle fullscreen window (need to select a monitor).
        let window_mode = glfw::WindowMode::Windowed;
        if fullscreen {
            eprintln!("GLFW fullscreen window support not implemented!");
        }

        let (mut window, event_receiver) = glfw_instance
            .create_window(window_size.width as u32, window_size.height as u32, title.as_str(), window_mode)
            .expect("Failed to create GLFW window!");

        window.make_current();

        // Listen to these application events:
        window.set_size_polling(true);
        window.set_close_polling(true);
        window.set_key_polling(true);
        window.set_char_polling(true);
        window.set_scroll_polling(true);
        window.set_mouse_button_polling(true);

        // On MacOS this generates a lot of TTY spam about missing
        // OpenGL functions that we don't need or care about. This
        // is a hack to stop the TTY spamming but still keep a record
        // of the errors if ever required for inspection.
        utils::macos_redirect_stderr(|| {
            gl::load_with(|symbol| window.get_proc_address(symbol))
        }, "stderr_gl_load_app.log");

        Self {
            title: title,
            window_size: window_size,
            fullscreen: fullscreen,
            should_quit: false,
            glfw_instance: glfw_instance,
            window: window,
            event_receiver: event_receiver,
        }
    }
}

impl Application for GlfwApplication {
    fn should_quit(&self) -> bool {
        self.should_quit
    }

    fn request_quit(&mut self) {
        self.window.set_should_close(true);
        self.should_quit = true;
    }

    fn poll_events(&mut self) -> ApplicationEventList {
        self.glfw_instance.poll_events();

        let mut translated_events = ApplicationEventList::new();

        for (_, event) in glfw::flush_messages(&self.event_receiver) {
            // NOTE: To receive events here we must call set_<event>_polling().
            // See set_size_polling/set_close_polling calls above.
            match event {
                glfw::WindowEvent::Size(width, height) => {
                    self.window_size.width = width;
                    self.window_size.height = height;
                    translated_events.push(ApplicationEvent::WindowResize(Size2D::new(width, height)));
                }
                glfw::WindowEvent::Close => {
                    translated_events.push(ApplicationEvent::Quit);
                }
                glfw::WindowEvent::Key(key, _scan_code, action, modifiers) => {
                    translated_events.push(ApplicationEvent::KeyInput(key, action, modifiers));
                }
                glfw::WindowEvent::Char(c) => {
                    translated_events.push(ApplicationEvent::CharInput(c));
                }
                glfw::WindowEvent::Scroll(x, y) => {
                    translated_events.push(ApplicationEvent::Scroll(Vec2::new(x as f32, y as f32)));
                }
                glfw::WindowEvent::MouseButton(button, action, modifiers) => {
                    translated_events.push(ApplicationEvent::MouseButton(button, action, modifiers));
                }
                unhandled_event => {
                    eprintln!("Unhandled GLFW window event: {:?}", unhandled_event);
                }
            }
        }

        translated_events
    }

    fn present(&mut self) {
        self.window.swap_buffers();
    }

    fn window_size(&self) -> Size2D {
        self.window_size
    }

    fn framebuffer_size(&self) -> Size2D {
        let (width, height) = self.window.get_framebuffer_size();
        Size2D::new(width, height)
    }

    fn content_scale(&self) -> Vec2 {
        let (x_scale, y_scale) = self.window.get_content_scale();
        Vec2::new(x_scale, y_scale)
    }

    type InputSystemType = GlfwInputSystem;
    fn create_input_system(&self) -> GlfwInputSystem {
        GlfwInputSystem::new(self)
    }
}

// ----------------------------------------------
// Internal helpers
// ----------------------------------------------

#[inline]
fn get_glfw_window_ptr<T: Application>(app: &T) -> *mut glfw::PWindow {
    unsafe {
        // SAFETY: Type `T` is always GlfwApplication, there's only one implementation of the Application trait.
        debug_assert!(std::mem::size_of::<T>() == std::mem::size_of::<GlfwApplication>());
        let glfw_app_ptr = app as *const T as *const GlfwApplication;
        &(*glfw_app_ptr).window as *const glfw::PWindow as *mut glfw::PWindow
    }
}

// For the ImGui OpenGL backend.
pub fn load_gl_func<T: Application>(app: &T, func_name: &'static str) -> *const c_void {
    let window_ptr = get_glfw_window_ptr(app);
    debug_assert!(window_ptr.is_null() == false);
    unsafe { (*window_ptr).get_proc_address(func_name) as *const c_void }
}

// ----------------------------------------------
// GlfwInputSystem
// ----------------------------------------------

pub struct GlfwInputSystem {
    window_ptr: *const glfw::PWindow,
}

impl GlfwInputSystem {
    pub fn new<T: Application>(app: &T) -> Self {
        Self {
            // SAFETY: Application will persist for as long at InputSystem.
            window_ptr: get_glfw_window_ptr(app),
        }
    }

    #[inline]
    fn get_window(&self) -> &glfw::PWindow {
        debug_assert!(self.window_ptr.is_null() == false);
        unsafe { &(*self.window_ptr) }
    }
}

impl InputSystem for GlfwInputSystem {
    fn cursor_pos(&self) -> Point2D {
        let (x, y) = self.get_window().get_cursor_pos();
        Point2D::new(x as i32, y as i32)
    }

    fn mouse_button_state(&self, button: MouseButton) -> InputAction {
        self.get_window().get_mouse_button(button)
    }

    fn key_state(&self, key: InputKey) -> InputAction {
        self.get_window().get_key(key)
    }
}
