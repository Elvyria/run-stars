pub const BRAILE: &str = "⣧⣏⡟⠿⢻⣹⣼⣶";
pub const ARROW:  &str = "▹▸▹▹▹";

#[allow(dead_code)] 
pub const BOUNCE: &str = "⡀⠄⠂⠁⠈⠐⠠⢀⠠⠐⠈⠁⠂⠄";

pub struct Spinner {
    chars:   &'static str,
    size:    usize,
    current: usize,
}

impl Spinner {
    pub fn new(chars: &'static str) -> Self {
        Spinner {
            chars,
            size: chars.chars().next().unwrap().len_utf8(),
            current: 0,
        }
    }

    #[inline]
    pub fn next(&mut self) {
        self.current = (self.current + self.size) % self.chars.len();
    }

    pub fn current(&self) -> &'static str {
        // SAFETY: Spinner::next keeps index in bounds
        unsafe { self.chars.get_unchecked(self.current..self.current + self.size) }
    }
}
