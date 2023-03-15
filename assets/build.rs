use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

fn main() {
    println!("cargo:rerun-if-changed=shaders");

    let compiler = shaderc::Compiler::new().unwrap();
    let options = shaderc::CompileOptions::new().unwrap();
    let shader_source_paths = vec![
        PathBuf::from("shaders/vertex.glsl"),
        PathBuf::from("shaders/fragment.glsl"),
    ];
    let shader_output_paths: Vec<PathBuf> = shader_source_paths
        .iter()
        .map(|path| {
            PathBuf::from(format!(
                "shaders/compiled/{}.spv",
                path.file_stem().unwrap().to_str().unwrap()
            ))
        })
        .collect();

    std::iter::zip(shader_source_paths, shader_output_paths).for_each(|(input, output)| {
        let mut file = File::open(&input).unwrap();
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).unwrap();

        let source = String::from_utf8(buf).unwrap();

        let kind = if source.starts_with("VERTEX") {
            shaderc::ShaderKind::Vertex
        } else if source.starts_with("FRAGMENT") {
            shaderc::ShaderKind::Fragment
        } else {
            panic!("Unknown shader type in file {}", input.display())
        };

        let source = source.lines().skip(1).collect::<Vec<&str>>().join("\n");

        let spirv = compiler
            .compile_into_spirv(
                &source,
                kind,
                input.file_name().unwrap().to_str().unwrap(),
                "main",
                Some(&options),
            )
            .unwrap();

        let mut output_file = File::create(output).unwrap();
        output_file.write_all(spirv.as_binary_u8()).unwrap();
    });
}
