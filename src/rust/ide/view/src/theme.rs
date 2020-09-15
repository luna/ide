//! Application theme setup.

/// `define_theme` helper.
macro_rules! _define_theme_literals {
    ([$theme_name:ident $($path:ident)*] $name:ident = $e:expr) => {
        $theme_name.insert(stringify!($($path.)*.$name), $e);
    };

    ([$($path:ident)*] $name:ident = $e:expr; $($rest:tt)*) => {
        _define_theme_literals!([$($path)*] $name = $e);
        _define_theme_literals!([$($path)*] $($rest)*);
    };

    ([$($path:ident)*] $name:ident {$($t:tt)*}) => {
        _define_theme_literals!([$($path)* $name] $($t)*);
    };

    ([$($path:ident)*] $name:ident {$($t:tt)*} $($rest:tt)*) => {
        _define_theme_literals!([$($path)*] $name {$($t)*});
        _define_theme_literals!([$($path)*] $($rest)*);
    };
}

macro_rules! _define_theme_modules {
    ([$theme_name:ident $($path:ident)*] $name:ident = $e:expr) => {
        pub const $name : &str = stringify!($($path.)*.$name);
    };

    ([$($path:ident)*] $name:ident = $e:expr; $($rest:tt)*) => {
        _define_theme_modules!([$($path)*] $name = $e);
        _define_theme_modules!([$($path)*] $($rest)*);
    };

    ([$($path:ident)*] $name:ident {$($t:tt)*}) => {
        pub mod $name {
            _define_theme_modules!([$($path)* $name] $($t)*);
        }
    };

    ([$($path:ident)*] $name:ident {$($t:tt)*} $($rest:tt)*) => {
        _define_theme_modules!([$($path)*] $name {$($t)*});
        _define_theme_modules!([$($path)*] $($rest)*);
    };
}

/// Used to define default theme.
#[macro_export]
macro_rules! define_default_theme {
    ($name:ident $($t:tt)*) => {
        define_theme!($name $($t)*);

        #[allow(non_upper_case_globals)]
        #[allow(missing_docs)]
        pub mod vars {
            _define_theme_modules!([$name] $($t)*);
        }
    };
}

/// Used to define any theme.
#[macro_export]
macro_rules! define_theme {
    ($name:ident $($t:tt)*) => {
        #[allow(missing_docs)]
        pub mod $name {
            use ensogl::application::Application;
            use ensogl::data::color;
            use ensogl::display::style::theme;

            /// Setup the $name theme in application.
            pub fn setup(app:&Application) {
                let mut $name = theme::Theme::new();
                _define_theme_literals!([$name] $($t)*);
                app.themes.register(stringify!($name),$name);
            }
        }
    };
}

define_theme! { dark
    application {
        background {
            color = color::Lcha::new(0.13,0.013,0.18,1.0)
        }
        text {
            color = color::Lcha::new(1.0,0.0,0.0,0.7);
            selection {
                color = color::Lcha::new(0.7,0.0,0.125,0.7)
            }
        }
    }
    graph_editor {
        node {
            background {
                color = color::Lcha::new(0.2,0.013,0.18,1.0)
            }
            selection {
                color = color::Lcha::new(0.72,0.5,0.22,1.0);
                size = 7.0
            }
        }
        visualization {
            background {
                color = color::Lcha::new(0.2,0.013,0.18,1.0)
            }
        }
    }
    breadcrumbs {
        full {
            color = color::Lcha::new(1.0,0.0,0.0,0.7)
        }
        transparent {
            color = color::Lcha::new(1.0,0.0,0.0,0.4)
        }
        selected {
            color = color::Lcha::new(1.0,0.0,0.0,0.6)
        }
        deselected{
            left {
                color = color::Lcha::new(1.0,0.0,0.0,0.6)
            }
            right {
                color = color::Lcha::new(1.0,0.0,0.0,0.2)
            }
        }
        hover {
            color = color::Lcha::new(1.0,0.0,0.0,0.6)
        }
    }
    list_view {
        background {
            color = color::Lcha::new(0.2,0.013,0.18,1.0)
        }
        highlight {
            color = color::Lcha::new(0.72,0.5,0.22,1.0)
        }
    }
    edge {
        split_color {
            lightness_factor = 0.2;
            chroma_factor = 1.0
        }
    }
    _type {
        missing {
            color = color::Lcha::new(0.5,0.0,0.0,1.0)
        }
        color {
            luminance = 0.5;
            chroma = 0.8
        }
    }
}

define_default_theme! { light
    application {
        background {
            color = color::Lcha::new(0.96,0.013,0.18,1.0)
        }
        text {
            color = color::Lcha::new(0.0,0.0,0.0,0.7);
            selection {
                color = color::Lcha::new(0.7,0.0,0.125,0.7)
            }
        }
    }
    graph_editor {
        node {
            background {
                color = color::Lcha::new(0.98,0.013,0.18,1.0)
            }
            selection {
                color = color::Lcha::new(0.83,0.58,0.436,1.0);
                size = 7.0
            }
        }
        visualization {
            background {
                color = color::Lcha::new(0.98,0.013,0.18,1.0)
            }
        }
    }
    breadcrumbs {
        full {
            color = color::Lcha::new(0.0,0.0,0.0,0.7)
        }
        transparent {
            color = color::Lcha::new(0.0,0.0,0.0,0.4)
        }
        selected {
            color = color::Lcha::new(0.0,0.0,0.0,0.6)
        }
        deselected{
            left {
                color = color::Lcha::new(0.0,0.0,0.0,0.6)
            }
            right {
                color = color::Lcha::new(0.0,0.0,0.0,0.2)
            }
        }
        hover {
            color = color::Lcha::new(0.0,0.0,0.0,0.6)
        }
    }
    list_view {
        background {
            color = color::Lcha::new(0.98,0.013,0.18,1.0)
        }
        highlight {
            color = color::Lcha::new(0.55,0.65,0.79,1.0)
        }
    }
    edge {
        split_color {
            lightness_factor = 1.2;
            chroma_factor = 0.8
        }
    }
    _type {
        missing {
            color = color::Lcha::new(0.8,0.0,0.0,1.0)
        }
        color {
            luminance = 0.8;
            chroma = 0.6
        }
    }
}
