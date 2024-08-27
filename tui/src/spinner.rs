pub const BRAILE: &str = "⣧⣏⡟⠿⢻⣹⣼⣶";
pub const BOUNCE: &str = "⡀⠄⠂⠁⠈⠐⠠⢀⠠⠐⠈⠁⠂⠄";
pub const ARROW:  &str = "▹▸▹▹▹";

pub struct Spinner {
    chars: &'static str,
    size: usize,
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

    pub fn next(&mut self) {
        // SAFETY: char arrays are very small compared to usize::MAX
        unsafe {
            if self.current < self.chars.len().unchecked_sub(self.size) {
                self.current = self.current.unchecked_add(self.size);
            } else {
                self.current = 0;
            }
        }
    }

    pub fn current(&self) -> &'static str {
        // SAFETY: Spinner::next keeps index in bounds
        unsafe { self.chars.get_unchecked(self.current..self.current.unchecked_add(self.size)) }
    }
}
