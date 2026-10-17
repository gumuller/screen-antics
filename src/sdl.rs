//! Minimal bindings to the installed SDL2 ABI. No emulator or browser runtime.
use std::ffi::{c_char, c_int, c_void, CStr, CString};
#[link(name = "SDL2")]
extern "C" {
    fn SDL_Init(flags: u32) -> c_int;
    fn SDL_Quit();
    fn SDL_GetError() -> *const c_char;
    fn SDL_CreateWindow(
        title: *const c_char,
        x: c_int,
        y: c_int,
        w: c_int,
        h: c_int,
        flags: u32,
    ) -> *mut c_void;
    fn SDL_DestroyWindow(w: *mut c_void);
    fn SDL_CreateRenderer(w: *mut c_void, index: c_int, flags: u32) -> *mut c_void;
    fn SDL_DestroyRenderer(r: *mut c_void);
    fn SDL_GetRendererOutputSize(r: *mut c_void, w: *mut c_int, h: *mut c_int) -> c_int;
    fn SDL_SetHint(name: *const c_char, value: *const c_char) -> c_int;
    fn SDL_RenderSetLogicalSize(r: *mut c_void, w: c_int, h: c_int) -> c_int;
    fn SDL_CreateTexture(
        r: *mut c_void,
        format: u32,
        access: c_int,
        w: c_int,
        h: c_int,
    ) -> *mut c_void;
    fn SDL_DestroyTexture(t: *mut c_void);
    fn SDL_UpdateTexture(
        t: *mut c_void,
        rect: *const c_void,
        pixels: *const c_void,
        pitch: c_int,
    ) -> c_int;
    fn SDL_RenderClear(r: *mut c_void) -> c_int;
    fn SDL_RenderCopy(
        r: *mut c_void,
        t: *mut c_void,
        src: *const c_void,
        dst: *const c_void,
    ) -> c_int;
    fn SDL_RenderPresent(r: *mut c_void);
    fn SDL_PollEvent(e: *mut c_void) -> c_int;
    fn SDL_SetWindowFullscreen(w: *mut c_void, flags: u32) -> c_int;
    fn SDL_ShowCursor(toggle: c_int) -> c_int;
    fn SDL_SetWindowTitle(w: *mut c_void, title: *const c_char);
    fn SDL_OpenAudioDevice(
        name: *const c_char,
        capture: c_int,
        desired: *const AudioSpec,
        obtained: *mut AudioSpec,
        changes: c_int,
    ) -> u32;
    fn SDL_QueueAudio(device: u32, data: *const c_void, len: u32) -> c_int;
    fn SDL_ClearQueuedAudio(device: u32);
    fn SDL_PauseAudioDevice(device: u32, pause: c_int);
    fn SDL_CloseAudioDevice(device: u32);
}
#[repr(C)]
struct AudioSpec {
    freq: c_int,
    format: u16,
    channels: u8,
    silence: u8,
    samples: u16,
    padding: u16,
    size: u32,
    callback: Option<extern "C" fn(*mut c_void, *mut u8, c_int)>,
    userdata: *mut c_void,
}
pub enum Event {
    Quit,
    Key(i32),
    Mouse,
    Resize(u32, u32),
}
pub struct Display {
    window: *mut c_void,
    renderer: *mut c_void,
    texture: *mut c_void,
    full: bool,
    audio: u32,
    width: u32,
    height: u32,
}
fn error() -> String {
    unsafe {
        CStr::from_ptr(SDL_GetError())
            .to_string_lossy()
            .into_owned()
    }
}
impl Display {
    pub fn new(full: bool, width: u32, height: u32) -> Result<Self, String> {
        validate_size(width, height)?;
        unsafe {
            if SDL_Init(0x20 | 0x10) != 0 {
                return Err(error());
            }
            let title = CString::new("Screen Antics · Rust").unwrap();
            // A comfortable initial window with the requested framebuffer aspect.
            // Fullscreen desktop obtains the actual monitor pixel size below.
            let scale = (1280.0 / width as f64).min(900.0 / height as f64).min(1.0);
            let window = SDL_CreateWindow(
                title.as_ptr(),
                0x2fff0000,
                0x2fff0000,
                (width as f64 * scale).round().max(1.0) as c_int,
                (height as f64 * scale).round().max(1.0) as c_int,
                0x20 | 0x2000 | if full { 0x1001 } else { 0 },
            );
            if window.is_null() {
                SDL_Quit();
                return Err(error());
            }
            let mut renderer = SDL_CreateRenderer(window, -1, 2);
            if renderer.is_null() {
                renderer = SDL_CreateRenderer(window, -1, 1)
            }
            if renderer.is_null() {
                SDL_DestroyWindow(window);
                SDL_Quit();
                return Err(error());
            }
            // SDL centers and letterboxes logical output; it never stretches it.
            SDL_SetHint(
                b"SDL_RENDER_LOGICAL_SIZE_MODE\0".as_ptr().cast(),
                b"letterbox\0".as_ptr().cast(),
            );
            SDL_SetHint(
                b"SDL_RENDER_SCALE_QUALITY\0".as_ptr().cast(),
                b"nearest\0".as_ptr().cast(),
            );
            if SDL_RenderSetLogicalSize(renderer, width as c_int, height as c_int) != 0 {
                let message = error();
                SDL_DestroyRenderer(renderer);
                SDL_DestroyWindow(window);
                SDL_Quit();
                return Err(message);
            }
            let texture =
                SDL_CreateTexture(renderer, 0x16362004, 1, width as c_int, height as c_int);
            if texture.is_null() {
                SDL_DestroyRenderer(renderer);
                SDL_DestroyWindow(window);
                SDL_Quit();
                return Err(error());
            }
            let spec = AudioSpec {
                freq: 11025,
                format: 8,
                channels: 1,
                silence: 128,
                samples: 1024,
                padding: 0,
                size: 0,
                callback: None,
                userdata: std::ptr::null_mut(),
            };
            let audio = SDL_OpenAudioDevice(std::ptr::null(), 0, &spec, std::ptr::null_mut(), 0);
            if audio != 0 {
                SDL_PauseAudioDevice(audio, 0)
            }
            SDL_ShowCursor(if full { 0 } else { 1 });
            Ok(Self {
                window,
                renderer,
                texture,
                full,
                audio,
                width,
                height,
            })
        }
    }
    /// Actual output pixels, including the monitor's high DPI scale factor.
    pub fn drawable_size(&self) -> (u32, u32) {
        let (mut width, mut height) = (0, 0);
        unsafe {
            if SDL_GetRendererOutputSize(self.renderer, &mut width, &mut height) == 0
                && width > 0
                && height > 0
            {
                return (width as u32, height as u32);
            }
        }
        (self.width, self.height)
    }
    /// Change framebuffer resolution without replacing the window or audio.
    pub fn resize_surface(&mut self, width: u32, height: u32) -> Result<(), String> {
        validate_size(width, height)?;
        if (width, height) == (self.width, self.height) {
            return Ok(());
        }
        unsafe {
            let texture = SDL_CreateTexture(
                self.renderer,
                0x16362004,
                1,
                width as c_int,
                height as c_int,
            );
            if texture.is_null() {
                return Err(error());
            }
            if SDL_RenderSetLogicalSize(self.renderer, width as c_int, height as c_int) != 0 {
                let message = error();
                SDL_DestroyTexture(texture);
                return Err(message);
            }
            SDL_DestroyTexture(self.texture);
            self.texture = texture;
        }
        self.width = width;
        self.height = height;
        Ok(())
    }
    pub fn title(&self, title: &str) {
        if let Ok(t) = CString::new(title) {
            unsafe { SDL_SetWindowTitle(self.window, t.as_ptr()) }
        }
    }
    pub fn show(&self, pixels: &[u32]) -> Result<(), String> {
        if pixels.len() != self.width as usize * self.height as usize {
            return Err("Invalid framebuffer length".into());
        }
        unsafe {
            if SDL_UpdateTexture(
                self.texture,
                std::ptr::null(),
                pixels.as_ptr().cast(),
                self.width as c_int * 4,
            ) != 0
            {
                return Err(error());
            }
            if SDL_RenderClear(self.renderer) != 0 {
                return Err(error());
            }
            if SDL_RenderCopy(
                self.renderer,
                self.texture,
                std::ptr::null(),
                std::ptr::null(),
            ) != 0
            {
                return Err(error());
            }
            SDL_RenderPresent(self.renderer);
        }
        Ok(())
    }
    pub fn events(&self) -> Vec<Event> {
        let mut out = Vec::new();
        unsafe {
            let mut e = [0u64; 7];
            while SDL_PollEvent(e.as_mut_ptr().cast()) != 0 {
                let b = std::slice::from_raw_parts(e.as_ptr().cast::<u8>(), 56);
                match u32::from_ne_bytes(b[0..4].try_into().unwrap()) {
                    0x100 => out.push(Event::Quit),
                    // SDL_WINDOWEVENT_RESIZED / SIZE_CHANGED / DISPLAY_CHANGED.
                    // Query pixels, because event data is measured in window units.
                    0x200 if matches!(b[12], 5 | 6 | 18) => {
                        let (width, height) = self.drawable_size();
                        out.push(Event::Resize(width, height));
                    }
                    0x300 => {
                        if b[13] == 0 {
                            out.push(Event::Key(i32::from_ne_bytes(
                                b[20..24].try_into().unwrap(),
                            )))
                        }
                    }
                    0x400 | 0x401 => out.push(Event::Mouse),
                    _ => {}
                }
            }
        }
        out
    }
    pub fn fullscreen(&mut self) {
        unsafe {
            let full = !self.full;
            if SDL_SetWindowFullscreen(self.window, if full { 0x1001 } else { 0 }) == 0 {
                self.full = full;
                SDL_ShowCursor(if self.full { 0 } else { 1 });
            }
        }
    }
    pub fn sound(&self, pcm: &[u8]) {
        if self.audio != 0 {
            unsafe {
                SDL_ClearQueuedAudio(self.audio);
                SDL_QueueAudio(self.audio, pcm.as_ptr().cast(), pcm.len() as u32);
            }
        }
    }
    pub fn silence(&self) {
        if self.audio != 0 {
            unsafe { SDL_ClearQueuedAudio(self.audio) }
        }
    }
}
fn validate_size(width: u32, height: u32) -> Result<(), String> {
    if width == 0
        || height == 0
        || width > 16_384
        || height > 16_384
        || width as u64 * height as u64 > 67_108_864
    {
        return Err("Framebuffer must be 1–16384 pixels per side, at most 64 megapixels".into());
    }
    Ok(())
}
impl Drop for Display {
    fn drop(&mut self) {
        unsafe {
            if self.audio != 0 {
                SDL_CloseAudioDevice(self.audio)
            }
            SDL_DestroyTexture(self.texture);
            SDL_DestroyRenderer(self.renderer);
            SDL_DestroyWindow(self.window);
            SDL_Quit();
        }
    }
}
