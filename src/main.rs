#![cfg_attr(feature = "clippy", feature(plugin))]
#![cfg_attr(feature = "clippy", plugin(clippy))]
#![allow(dead_code)]
#![allow(clippy::float_cmp)]

extern crate cgmath;
#[macro_use]
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate rayon;
extern crate rustlight;

use clap::{App, Arg, SubCommand};
use rustlight::integrators::IntegratorType;
fn match_infinity<T: std::str::FromStr>(input: &str) -> Option<T> {
    match input {
        "inf" => None,
        _ => match input.parse::<T>() {
            Ok(x) => Some(x),
            Err(_e) => panic!("wrong input for inf type parameter"),
        },
    }
}

fn main() {
    // Read input args
    let max_arg = Arg::with_name("max")
        .takes_value(true)
        .short("m")
        .default_value("inf");
    let min_arg = Arg::with_name("min")
        .takes_value(true)
        .short("n")
        .default_value("inf");
    let iterations_arg = Arg::with_name("iterations")
        .takes_value(true)
        .short("r")
        .default_value("50");
    let recons_type_arg = Arg::with_name("reconstruction_type")
        .takes_value(true)
        .short("t")
        .default_value("uniform");
    let matches =
        App::new("rustlight")
            .version("0.1.0")
            .author("Adrien Gruson <adrien.gruson@gmail.com>")
            .about("A Rusty Light Transport simulation program")
            .arg(
                Arg::with_name("scene")
                    .required(true)
                    .takes_value(true)
                    .index(1)
                    .help("JSON file description"),
            )
            .arg(Arg::with_name("average").short("a").takes_value(true).help(
                "average several pass of the integrator with a time limit ('inf' is possible)",
            ))
            .arg(
                Arg::with_name("nbthreads")
                    .takes_value(true)
                    .short("t")
                    .default_value("auto")
                    .help("number of thread for the computation"),
            )
            .arg(
                Arg::with_name("image_scale")
                    .takes_value(true)
                    .short("s")
                    .default_value("1.0")
                    .help("image scaling factor"),
            )
            .arg(
                Arg::with_name("output")
                    .takes_value(true)
                    .short("o")
                    .help("output image file"),
            )
            .arg(Arg::with_name("debug").short("d").help("debug output"))
            .arg(
                Arg::with_name("nbsamples")
                    .short("n")
                    .takes_value(true)
                    .help("integration technique"),
            )
            .subcommand(
                SubCommand::with_name("path")
                    .about("path tracing")
                    .arg(&max_arg)
                    .arg(&min_arg)
                    .arg(
                        Arg::with_name("primitive")
                            .short("p")
                            .help("do not use next event estimation"),
                    ),
            )
            .subcommand(
                SubCommand::with_name("gradient-path")
                    .about("gradient path tracing")
                    .arg(&max_arg)
                    .arg(&min_arg)
                    .arg(&iterations_arg)
                    .arg(&recons_type_arg),
            )
            .subcommand(
                SubCommand::with_name("gradient-path-explicit")
                    .about("gradient path tracing")
                    .arg(&max_arg)
                    .arg(&min_arg)
                    .arg(&iterations_arg)
                    .arg(&recons_type_arg)
                    .arg(
                        Arg::with_name("min_survival")
                            .takes_value(true)
                            .short("s")
                            .default_value("1.0"),
                    ),
            )
            .subcommand(
                SubCommand::with_name("pssmlt")
                    .about("path tracing with MCMC sampling")
                    .arg(&max_arg)
                    .arg(&min_arg)
                    .arg(
                        Arg::with_name("large_prob")
                            .takes_value(true)
                            .short("p")
                            .default_value("0.3"),
                    ),
            )
            .subcommand(
                SubCommand::with_name("path-explicit")
                    .about("path tracing with explict light path construction")
                    .arg(&max_arg)
                    .arg(
                        Arg::with_name("strategy")
                            .takes_value(true)
                            .short("s")
                            .default_value("all"),
                    ),
            )
            .subcommand(
                SubCommand::with_name("light-explicit")
                    .about("light tracing with explict light path construction")
                    .arg(&max_arg),
            )
            .subcommand(
                SubCommand::with_name("vpl")
                    .about("brute force virtual point light integrator")
                    .arg(&max_arg)
                    .arg(
                        Arg::with_name("clamping")
                            .takes_value(true)
                            .short("b")
                            .default_value("0.0"),
                    )
                    .arg(
                        Arg::with_name("nb_vpl")
                            .takes_value(true)
                            .short("n")
                            .default_value("128"),
                    ),
            )
            .subcommand(
                SubCommand::with_name("ao")
                    .about("ambiant occlusion")
                    .arg(
                        Arg::with_name("distance")
                            .takes_value(true)
                            .short("d")
                            .default_value("inf"),
                    )
                    .arg(
                        Arg::with_name("normal-correction")
                            .takes_value(false)
                            .short("n"),
                    ),
            )
            .subcommand(
                SubCommand::with_name("direct")
                    .about("direct lighting")
                    .arg(
                        Arg::with_name("bsdf")
                            .takes_value(true)
                            .short("b")
                            .default_value("1"),
                    )
                    .arg(
                        Arg::with_name("light")
                            .takes_value(true)
                            .short("l")
                            .default_value("1"),
                    ),
            )
            .get_matches();

    /////////////// Setup logging system
    if matches.is_present("debug") {
        // FIXME: add debug flag?
        env_logger::Builder::from_default_env()
            .default_format_timestamp(false)
            .init();
    } else {
        env_logger::Builder::from_default_env()
            .default_format_timestamp(false)
            .parse_filters("info")
            .init();
    }
    /////////////// Check output extension
    let imgout_path_str = matches.value_of("output").unwrap_or("test.pfm");

    //////////////// Load the rendering configuration
    let nb_samples = value_t_or_exit!(matches.value_of("nbsamples"), usize);

    //////////////// Load the scene
    let scene = matches
        .value_of("scene")
        .expect("no scene parameter provided");
    let scene = rustlight::scene_loader::SceneLoaderManager::default()
        .load(scene.to_string())
        .expect("error on loading the scene");
    let scene = match matches.value_of("nbthreads").unwrap() {
        "auto" => scene,
        x => {
            let v = x.parse::<usize>().expect("Wrong number of thread");
            if v == 0 {
                panic!("Impossible to use 0 thread for the computation");
            }
            scene.nb_threads(v)
        }
    };
    let mut scene = scene.nb_samples(nb_samples).output_img(imgout_path_str);

    ///////////////// Tweak the image size
    {
        let image_scale = value_t_or_exit!(matches.value_of("image_scale"), f32);
        if image_scale != 1.0 {
            info!("Scale the image: {:?}", image_scale);
            assert!(image_scale != 0.0);
            scene.camera.scale_image(image_scale);
        }
    }

    ///////////////// Create the main integrator
    let mut int = match matches.subcommand() {
        ("ao", Some(m)) => {
            let normal_correction = m.is_present("normal-correction");
            let dist = match_infinity(m.value_of("distance").unwrap());
            IntegratorType::Primal(Box::new(rustlight::integrators::ao::IntegratorAO {
                max_distance: dist,
                normal_correction,
            }))
        }
        _ => panic!("unknown integrator"),
    };
    let img = if matches.is_present("average") {
        // Add an additional wraping on the struct
        let time_out = match_infinity(matches.value_of("average").unwrap());
        let mut int =
            IntegratorType::Primal(Box::new(rustlight::integrators::avg::IntegratorAverage {
                time_out,
                integrator: int,
            }));
        int.compute(&scene)
    } else {
        int.compute(&scene)
    };

    // Save the image
    img.save("primal", imgout_path_str);
}
