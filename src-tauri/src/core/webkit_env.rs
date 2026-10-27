use crate::core::keyboard_driver::Platform;

pub const DEFAULTS: [(&str, &str); 1] = [("WEBKIT_DISABLE_DMABUF_RENDERER", "1")];

pub fn apply_defaults(
    platform: Platform,
    is_set: impl Fn(&str) -> bool,
    mut set: impl FnMut(&str, &str),
) {
    if platform != Platform::Linux {
        return;
    }
    for (name, value) in DEFAULTS {
        if !is_set(name) {
            set(name, value);
        }
    }
}

pub fn apply_process_defaults() {
    apply_defaults(
        Platform::current(),
        |name| std::env::var_os(name).is_some(),
        |name, value| std::env::set_var(name, value),
    );
}

#[cfg(test)]
mod tests;
