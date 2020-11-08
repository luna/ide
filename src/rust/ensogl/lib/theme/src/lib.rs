//! Application theme setup.

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]
#![warn(unused_qualifications)]
#![warn(unsafe_code)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]



// ==============
// === Macros ===
// ==============

/// `define_theme` helper.
#[macro_export]
macro_rules! _define_theme_literals {
    ($theme:ident [$($path:ident)*]) => {};
    ($theme:ident [$($path:ident)*] $qual:ident . $($var:ident).+ = $e:expr $(;$($rest:tt)*)?) => {
        $crate::_define_theme_literals!{$theme [$($path)*] $qual { $($var).+ = $e } $($($rest)*)?}
    };
    ($theme:ident [$($path:ident)*] $var:ident = $e:expr $(;$($rest:tt)*)?) => {
        $theme.insert(stringify!($($path.)*$var), $e);
        $crate::_define_theme_literals!{$theme [$($path)*] $($($rest)*)?}
    };
    ($theme:ident [$($path:ident)*] $path_segment:ident {$($t:tt)*} $($rest:tt)*) => {
        $crate::_define_theme_literals!{$theme [$($path)* $path_segment] $($t)*}
        $crate::_define_theme_literals!{$theme [$($path)*] $($rest)*}
    };
}

/// `define_theme` helper.
#[macro_export]
macro_rules! _define_theme_modules {
    ($theme:ident [$($path:ident)*]) => {};
    ($theme:ident [$($path:ident)*] $qual:ident . $($var:ident).+ = $e:expr $(;$($rest:tt)*)?) => {
        $crate::_define_theme_modules!{$theme [$($path)*] $qual {$($var).+ = $e} $($($rest)*)?}
    };
    ($theme:ident [$($path:ident)*] $var:ident = $e:expr $(;$($rest:tt)*)?) => {
        pub const $var : &str = stringify!($($path.)*$var);
        $crate::_define_theme_modules!{$theme [$($path)*] $($($rest)*)?}
    };
    ($theme:ident [$($path:ident)*] $path_segment:ident {$($t:tt)*} $($rest:tt)*) => {
        pub const $path_segment : &str = stringify!($($path.)*$path_segment);
        pub mod $path_segment {
            $crate::_define_theme_modules!{$theme [$($path)* $path_segment] $($t)*}
        }
        $crate::_define_theme_modules!{$theme [$($path)*] $($rest)*}
    };
}

/// Used to define default theme. This one aside from generating code for `StyleManager` also creates
/// nested public modules that makes accessing values much better than with bare string literals.
/// It adds the `var` module with string constants, so now, instead of having to get data by string
/// literal - like `style.get("foo.bar.baz",fallback)`, you can do
/// `style.get(theme::foo::bar::baz,fallback)`.
#[macro_export]
macro_rules! define_default_theme {
    ($name:ident $($t:tt)*) => {
        define_theme!{$name $($t)*}

        #[allow(non_upper_case_globals)]
        #[allow(missing_docs)]
        #[allow(non_snake_case)]
        pub mod vars {
            $crate::_define_theme_modules!{$name [] $($t)*}
        }
        pub use vars::*;
    };
}

/// Generates code for `StyleManager` from given cascade style definition. It generates module equal
/// to the theme name and there will be function `setup` which creates a theme definition in `app`.
#[macro_export]
macro_rules! define_theme {
    ($name:ident $($t:tt)*) => {
        #[allow(missing_docs)]
        #[allow(non_snake_case)]
        pub mod $name {
            use ensogl_core::application::Application;
            use ensogl_core::data::color::Lcha;
            use ensogl_core::display::style::theme;

            /// Setup the `$name` theme in application.
            pub fn setup(app:&Application) {
                let mut $name = theme::Theme::new();
                $crate::_define_theme_literals!{$name [] $($t)*}
                app.themes.register(stringify!($name),$name);
                app.themes.set_enabled(&[stringify!($name)]);
            }
        }
    };
}



// =============================
// === Light Theme & Modules ===
// =============================

define_default_theme! { light_theme
    application {
        background {
            color = Lcha(0.96,0.013,0.18,1.0);
        }
    }
    syntax {
        missing.color = Lcha(0.8,0.0,0.0,1.0);
        luminance     = 0.8;
        chroma        = 0.6;
        types {
            Text.hue = 0.22;
        }
    }
    text_editor {
        text {
            color           = Lcha(0.0,0.0,0.0,0.7);
            selection.color = Lcha(0.7,0.0,0.125,0.7);
        }
    }
    graph_editor {
        node {
            background {
                color          = Lcha(0.98,0.013,0.18,1.0);
                variant.dimmed = Lcha(0.98,0.013,0.18,1.0);
            }
            shadow {
                color        = Lcha(0.0,0.0,0.0,0.20);
                fading_color = Lcha(0.0,0.0,0.0,0.0);
                exponent     = 2.0;
            }
            selection {
                color = Lcha(0.83,0.58,0.436,1.0);
                size  = 7.0;
            }
            text {
                color             = Lcha(0.0,0.0,0.0,0.7);
                missing_arg_color = Lcha(0.0,0.0,0.0,0.3);
                variant.dimmed    = Lcha(0.7,0.0,0.0,0.7);
                selection.color   = Lcha(0.7,0.0,0.125,0.7);
            }
            actions {
                icon {
                    color          = Lcha(0.0,0.0,0.0,0.7);
                    variant.dimmed = Lcha(0.7,0.0,0.0,0.7);
                }
            }
        }
        visualization {
            background.color = Lcha(0.98,0.013,0.18,1.0);
            shadow {
                color        = Lcha(0.0,0.0,0.0,0.20);
                fading_color = Lcha(0.0,0.0,0.0,0.0);
                exponent     = 2.0;
                html {
                    alpha = 0.16;
                    size  = 16.0;
                }
            }
            text {
                color           = Lcha(0.0,0.0,0.0,0.7);
                selection.color = Lcha(0.7,0.0,0.125,0.7);
            }
            action_bar {
                background.color = Lcha(0.94,0.013,0.18,1.0);
                icon.color       = Lcha(0.0,0.0,0.0,0.7);
                text.color       = Lcha(0.0,0.0,0.0,0.7);
            }
        }
        breadcrumbs {
            full.color        = Lcha(0.0,0.0,0.0,0.7);
            transparent.color = Lcha(0.0,0.0,0.0,0.4);
            selected.color    = Lcha(0.0,0.0,0.0,0.7);
            hover.color       = Lcha(0.0,0.0,0.0,0.7);
            deselected  {
                left.color  = Lcha(0.0,0.0,0.0,0.5);
                right.color = Lcha(0.0,0.0,0.0,0.2);
            }
        }
        edge {
            split_color {
                lightness_factor = 1.2;
                chroma_factor    = 0.8;
            }
        }
    }
    widget {
        list_view {
            background.color = Lcha(0.98,0.013,0.18,1.0);
            highlight.color  = Lcha(0.83,0.58,0.436,1.0);
            shadow {
                color        = Lcha(0.0,0.0,0.0,0.20);
                fading_color = Lcha(0.0,0.0,0.0,0.0);
                exponent     = 2.0;
            }
            text {
                color           = Lcha(0.0,0.0,0.0,0.7);
                highlight.color = Lcha(0.8,0.0,0.0,1.0);
                selection.color = Lcha(0.7,0.0,0.125,0.7);
            }
        }
    }
    colors {
        dimming {
            lightness_factor = 1.1;
            chroma_factor    = 0.2;
        }
    }
}



// ==================
// === Dark Theme ===
// ==================

define_theme! { dark_theme
    application {
        background {
            color = Lcha(0.13,0.013,0.18,1.0);
        }
    }
    syntax {
        missing.color = Lcha(0.5,0.0,0.0,1.0);
        luminance     = 0.75;
        chroma        = 0.4;
        types {
            Text.hue   = 0.217;
            Number.hue = 0.68;
        }
    }
    text_editor {
        text {
            color           = Lcha(1.0,0.0,0.0,0.7);
            selection.color = Lcha(0.7,0.0,0.125,0.7);
        }
    }
    graph_editor {
        node {
            background {
                color          = Lcha(0.2,0.013,0.18,1.0);
                variant.dimmed = Lcha(0.15,0.013,0.18,1.0);
            }
            shadow {
                color        = Lcha(0.0,0.0,0.0,0.20);
                fading_color = Lcha(0.0,0.0,0.0,0.0);
                exponent     = 2.0;
            }
            selection {
                color = Lcha(0.72,0.5,0.22,1.0);
                size  = 7.0;
            }
            text {
                color             = Lcha(1.0,0.0,0.0,0.7);
                missing_arg_color = Lcha(1.0,0.0,0.0,0.3);
                variant.dimmed    = Lcha(0.25,0.013,0.18,1.0);
                selection.color   = Lcha(0.7,0.0,0.125,0.7);
            }
            actions {
                icon {
                    color = Lcha(1.0,0.0,0.0,0.7);
                    variant.dimmed = Lcha(0.4,0.00,0.0,1.0);
                }
            }
        }
        visualization {
            background.color = Lcha(0.2,0.013,0.18,1.0);
            shadow {
                color        = Lcha(0.0,0.0,0.0,0.20);
                fading_color = Lcha(0.0,0.0,0.0,0.0);
                exponent     = 2.0;
                html {
                    alpha = 0.16;
                    size  = 16.0
                }
            }
            text {
                color           = Lcha(1.0,0.0,0.0,0.7);
                selection.color = Lcha(0.7,0.0,0.125,0.7);
            }
            action_bar {
                background.color = Lcha(0.3,0.013,0.18,1.0);
                icon.color       = Lcha(1.0,0.0,0.0,0.7);
                text.color       = Lcha(1.0,0.0,0.0,0.7);
            }
        }
        breadcrumbs {
            full.color        = Lcha(1.0,0.0,0.0,0.7);
            transparent.color = Lcha(1.0,0.0,0.0,0.4);
            selected.color    = Lcha(1.0,0.0,0.0,0.7);
            hover.color       = Lcha(1.0,0.0,0.0,0.7);
            deselected  {
                left.color  = Lcha(1.0,0.0,0.0,0.5);
                right.color = Lcha(1.0,0.0,0.0,0.2);
            }
        }
        edge {
            split_color {
                lightness_factor = 0.2;
                chroma_factor    = 1.0;
            }
        }
    }
    widget {
        list_view {
            background.color = Lcha(0.2,0.013,0.18,1.0);
            highlight.color  = Lcha(0.72,0.5,0.22,1.0);
            shadow {
                color        = Lcha(0.0,0.0,0.0,0.20);
                fading_color = Lcha(0.0,0.0,0.0,0.0);
                exponent     = 2.0;
            }
            text {
                color           = Lcha(1.0,0.0,0.0,0.7);
                highlight.color = Lcha(0.7,0.0,0.0,1.0);
                selection.color = Lcha(0.7,0.0,0.125,0.7);
            }
        }
    }
    shadow {
        color        = Lcha(0.0,0.0,0.0,0.20);
        fading_color = Lcha(0.0,0.0,0.0,0.0);
        exponent     = 2.0;
    }
    colors {
        dimming {
            lightness_factor = 0.8;
            chroma_factor    = 0.2;
        }
    }
}
