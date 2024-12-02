extern crate argparse;
extern crate html2text;
use argparse::{ArgumentParser, Store, StoreOption, StoreTrue};
use html2text::config::{self, Config};
use html2text::render::{TextDecorator, TrivialDecorator};
use log::trace;
use std::io;
use std::io::Write;

use html2text::render::RichAnnotation;
fn default_colour_map(
    annotations: &[RichAnnotation],
    s: &str,
    use_css_colours: bool,
    no_default_colours: bool,
    hyperlinks: bool,
) -> String {
    use yansi::{hyperlink::HyperlinkExt, Paint};
    use RichAnnotation::*;
    // Explicit CSS colours override any other colours
    let mut have_explicit_colour = no_default_colours;
    let mut styled = s.to_string();
    trace!("default_colour_map: str={s}, annotations={annotations:?}");
    for annotation in annotations.iter() {
        styled = match annotation {
            Default => styled,
            Link(url) => {
                if hyperlinks {
                    styled.link(url).blue().underline().to_string()
                } else if !have_explicit_colour {
                    styled.blue().underline().to_string()
                } else {
                    styled
                }
            }
            Image(img) => {
                if hyperlinks {
                    styled.underline().blue().italic().link(img).to_string()
                } else if !have_explicit_colour {
                    styled.yellow().italic().to_string()
                } else {
                    styled
                }
            }
            Emphasis => styled.italic().to_string(),
            Strong => {
                if !have_explicit_colour {
                    styled.bold().to_string()
                } else {
                    styled
                }
            }
            Strikeout => {
                if !have_explicit_colour {
                    styled.strike().to_string()
                } else {
                    styled
                }
            }
            Code => {
                if !have_explicit_colour {
                    styled.blue().to_string()
                } else {
                    styled
                }
            }
            Preformat(_) => {
                if !have_explicit_colour {
                    styled.blue().to_string()
                } else {
                    styled
                }
            }
            Colour(c) => {
                if use_css_colours {
                    have_explicit_colour = true;
                    styled.rgb(c.r, c.g, c.b).to_string()
                } else {
                    styled
                }
            }
            BgColour(c) => {
                if use_css_colours {
                    styled.on_rgb(c.r, c.g, c.b).to_string()
                } else {
                    styled
                }
            }
            _ => styled,
        }
    }
    // Reverse the finish sequences
    trace!("default_colour_map: output={styled}");
    styled
}

fn update_config<T: TextDecorator>(mut config: Config<T>, flags: &Flags) -> Config<T> {
    if let Some(wrap_width) = flags.wrap_width {
        config = config.max_wrap_width(wrap_width);
    }
    #[cfg(feature = "css")]
    if flags.use_css {
        config = config.use_doc_css();
    }
    config
}

fn translate<R>(input: R, flags: Flags, literal: bool) -> String
where
    R: io::Read,
{
    {
        if flags.use_colour {
            let conf = config::rich();
            let conf = update_config(conf, &flags);
            #[cfg(feature = "css")]
            let use_css_colours = !flags.ignore_css_colours;
            #[cfg(not(feature = "css"))]
            let use_css_colours = false;
            #[cfg(feature = "css")]
            let use_only_css = flags.use_only_css;
            #[cfg(not(feature = "css"))]
            let use_only_css = false;
            return conf
                .coloured(input, flags.width, move |anns, s| {
                    default_colour_map(anns, s, use_css_colours, use_only_css, flags.hyperlinks)
                })
                .unwrap();
        }
    }
    #[cfg(feature = "css")]
    {
        if flags.show_css {
            let conf = config::plain();
            let conf = update_config(conf, &flags);
            let dom = conf.parse_html(input).unwrap();
            return html2text::dom_to_parsed_style(&dom).expect("Parsing CSS");
        }
    }
    if flags.show_dom {
        let conf = config::plain();
        let conf = update_config(conf, &flags);
        let dom = conf.parse_html(input).unwrap();
        dom.as_dom_string()
    } else if flags.show_render {
        let conf = config::plain();
        let conf = update_config(conf, &flags);
        let dom = conf.parse_html(input).unwrap();
        let rendertree = conf.dom_to_render_tree(&dom).unwrap();
        rendertree.to_string()
    } else if literal {
        let conf = config::with_decorator(TrivialDecorator::new());
        let conf = update_config(conf, &flags);
        conf.string_from_read(input, flags.width).unwrap()
    } else {
        let conf = config::plain();
        let conf = update_config(conf, &flags);
        conf.string_from_read(input, flags.width).unwrap()
    }
}

#[derive(Debug)]
struct Flags {
    width: usize,
    wrap_width: Option<usize>,
    #[allow(unused)]
    use_colour: bool,
    #[cfg(feature = "css")]
    use_css: bool,
    #[cfg(feature = "css")]
    ignore_css_colours: bool,
    #[cfg(feature = "css")]
    use_only_css: bool,
    show_dom: bool,
    show_render: bool,
    #[cfg(feature = "css")]
    show_css: bool,
    hyperlinks: bool,
}

fn main() {
    #[cfg(feature = "html_trace")]
    env_logger::init();

    let mut infile: Option<String> = None;
    let mut outfile: Option<String> = None;
    let mut flags = Flags {
        width: 80,
        wrap_width: None,
        use_colour: false,
        #[cfg(feature = "css")]
        use_css: false,
        #[cfg(feature = "css")]
        ignore_css_colours: false,
        #[cfg(feature = "css")]
        use_only_css: false,
        show_dom: false,
        show_render: false,
        #[cfg(feature = "css")]
        show_css: false,
        hyperlinks: false,
    };
    let mut literal: bool = false;

    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut infile).add_argument(
            "infile",
            StoreOption,
            "Input HTML file (default is standard input)",
        );
        ap.refer(&mut flags.width).add_option(
            &["-w", "--width"],
            Store,
            "Column width to format to (default is 80)",
        );
        ap.refer(&mut flags.wrap_width).add_option(
            &["-W", "--wrap-width"],
            StoreOption,
            "Maximum text wrap width (default same as width)",
        );
        ap.refer(&mut outfile).add_option(
            &["-o", "--output"],
            StoreOption,
            "Output file (default is standard output)",
        );
        ap.refer(&mut literal).add_option(
            &["-L", "--literal"],
            StoreTrue,
            "Output only literal text (no decorations)",
        );
        ap.refer(&mut flags.use_colour).add_option(
            &["--colour"],
            StoreTrue,
            "Use ANSI terminal colours",
        );
        #[cfg(feature = "css")]
        ap.refer(&mut flags.use_css)
            .add_option(&["--css"], StoreTrue, "Enable CSS");
        #[cfg(feature = "css")]
        ap.refer(&mut flags.ignore_css_colours)
            .add_option(&["--ignore-css-colour"], StoreTrue, "With --css, ignore CSS colour information (still hides elements with e.g. display: none)");
        #[cfg(feature = "css")]
        ap.refer(&mut flags.use_only_css).add_option(
            &["--only-css"],
            StoreTrue,
            "Don't use default non-CSS colours",
        );
        ap.refer(&mut flags.show_dom).add_option(
            &["--show-dom"],
            StoreTrue,
            "Show the parsed HTML DOM instead of rendered output",
        );
        ap.refer(&mut flags.show_render).add_option(
            &["--show-render"],
            StoreTrue,
            "Show the computed render tree instead of the rendered output",
        );
        #[cfg(feature = "css")]
        ap.refer(&mut flags.show_css).add_option(
            &["--show-css"],
            StoreTrue,
            "Show the parsed CSS instead of rendered output",
        );
        ap.refer(&mut flags.hyperlinks).add_option(
            &["--hyperlinks"],
            StoreTrue,
            "Show clickable, proper hyperlinks",
        );
        ap.parse_args_or_exit();
    }

    let data = match infile {
        None => {
            let stdin = io::stdin();

            translate(&mut stdin.lock(), flags, literal)
        }
        Some(name) => {
            let mut file = std::fs::File::open(name).expect("Tried to open file");
            translate(&mut file, flags, literal)
        }
    };

    match outfile {
        None => {
            print!("{}", data);
        }
        Some(name) => {
            let mut file = std::fs::File::create(name).expect("Tried to create file");
            write!(file, "{}", data).unwrap();
        }
    };
}
