#![feature(let_chains)]

use assets_build::{Model, Shader, Texture};

use walkdir::WalkDir;

fn main() {
    std::fs::create_dir_all("./src/shaders").unwrap();

    WalkDir::new(".")
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .iter()
                .find(|segment| segment.to_str().unwrap() == "out")
                .is_none()
        })
        .for_each(|entry| {
            let path = entry.path();
            path.extension()
                .map(|extension| extension.to_str())
                .flatten()
                .map(|extension| {
                    if ["glsl", "glb", "jpg", "jpeg", "png"].contains(&extension) {
                        println!("cargo:rerun-if-changed={}", path.display());
                    }

                    match extension {
                        "glsl" => Shader::new(path, &std::fs::read(path).unwrap())
                            .compile()
                            .codegen(),
                        "glb" => {
                            Model::new(path, &std::fs::read(path).unwrap());
                        }
                        "jpg" | "jpeg" | "png" => {
                            Texture::new(path, &std::fs::read(path).unwrap()).compile()
                        }
                        _ => (),
                    };
                });
        });

    let shader_modules = std::fs::read_dir("src/shaders")
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            entry
                .path()
                .extension()
                .map(|extension| {
                    if extension.to_str().unwrap() == "rs" {
                        entry
                            .path()
                            .file_stem()
                            .map(|osstr| osstr.to_str().unwrap().to_owned())
                    } else {
                        None
                    }
                })
                .flatten()
        })
        .filter(|modname| modname != "mod")
        .map(|modname| format!("mod {0};\npub use {0}::*;", modname))
        .collect::<Vec<String>>()
        .join("\n");

    std::fs::write("./src/shaders/mod.rs", shader_modules).unwrap();
}
