use image::imageops;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const RESOURCES: &str = "resources/sprites";

fn numeric_png_files(dir: &str) -> Vec<PathBuf> {
    let mut files: Vec<(usize, PathBuf)> = fs::read_dir(dir)
        .unwrap()
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let index = path.file_stem()?.to_str()?.parse().ok()?;
            (path.extension()?.to_str()? == "png").then_some((index, path))
        })
        .collect();
    files.sort_by_key(|(index, _)| *index);
    for (expected, (actual, path)) in files.iter().enumerate() {
        assert_eq!(
            expected,
            *actual,
            "numbered sprites in {dir} must be contiguous from 0; missing {expected} before {}",
            path.display()
        );
    }
    files
        .into_iter()
        .map(|(_, path)| fs::canonicalize(path).unwrap())
        .collect()
}

fn numeric_sprite_upper_bound(files: &[PathBuf]) -> usize {
    files
        .len()
        .checked_sub(1)
        .expect("at least one numbered sprite is required")
}

fn fallback_file(dir: &str) -> PathBuf {
    fs::canonicalize(Path::new(dir).join("n.png")).unwrap()
}

fn include_bytes_slice(name: &str, files: &[PathBuf]) -> String {
    let mut res = format!("pub const {name}: &[&[u8]] = &[\n");
    for file in files {
        res.push_str(&format!(
            "    include_bytes!(r#\"{}\"#),\n",
            file.to_str().unwrap()
        ));
    }
    res.push_str("];\n");
    res
}

fn include_bytes_const(name: &str, file: &Path) -> String {
    format!(
        "pub const {name}: &[u8] = include_bytes!(r#\"{}\"#);\n",
        file.to_str().unwrap()
    )
}

fn agent_files() -> String {
    let dir = format!("{RESOURCES}/agents");
    let files = numeric_png_files(&dir);
    let mut res = format!(
        "pub const MAX_NUMBERED_AGENT_SPRITE_ID: usize = {};\n",
        numeric_sprite_upper_bound(&files)
    );
    res.push_str(&include_bytes_slice("AGENT_BYTES", &files));
    res.push_str(&include_bytes_const(
        "AGENT_FALLBACK_BYTES",
        &fallback_file(&dir),
    ));
    res
}

fn generated_sprite_dir(out_dir: &Path, name: &str) -> PathBuf {
    let dir = out_dir.join("generated_sprites").join(name);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn rotate_file(source: &Path, destination_dir: &Path, rotations: u8) -> PathBuf {
    let mut image = image::open(source).unwrap().to_rgba8();
    for _ in 0..rotations {
        image = imageops::rotate90(&image);
    }

    let destination = destination_dir.join(source.file_name().unwrap());
    image.save(&destination).unwrap();
    fs::canonicalize(destination).unwrap()
}

fn rotated_files(source_dir: &str, destination_dir: &Path, rotations: u8) -> Vec<PathBuf> {
    numeric_png_files(source_dir)
        .into_iter()
        .map(|source| rotate_file(&source, destination_dir, rotations))
        .collect()
}

fn rotated_fallback_file(source_dir: &str, destination_dir: &Path, rotations: u8) -> PathBuf {
    rotate_file(&fallback_file(source_dir), destination_dir, rotations)
}

fn laser_files(out_dir: &Path) -> String {
    let dir = format!("{RESOURCES}/lasers");
    let horizontal_files = numeric_png_files(&dir);
    let vertical_dir = generated_sprite_dir(out_dir, "lasers_vertical");
    let vertical_files = rotated_files(&dir, &vertical_dir, 1);

    let mut res = include_bytes_slice("HORIZONTAL_LASER_BYTES", &horizontal_files);
    res.push_str(&include_bytes_const(
        "HORIZONTAL_LASER_FALLBACK_BYTES",
        &fallback_file(&dir),
    ));
    res.push_str(&include_bytes_slice(
        "VERTICAL_LASER_BYTES",
        &vertical_files,
    ));
    res.push_str(&include_bytes_const(
        "VERTICAL_LASER_FALLBACK_BYTES",
        &rotated_fallback_file(&dir, &vertical_dir, 1),
    ));
    res
}

fn laser_source_files(out_dir: &Path) -> String {
    let source_dir = format!("{RESOURCES}/sources");
    let mut res = include_bytes_slice("LASER_SOURCE_EAST_BYTES", &numeric_png_files(&source_dir));
    res.push_str(&include_bytes_const(
        "LASER_SOURCE_EAST_FALLBACK_BYTES",
        &fallback_file(&source_dir),
    ));

    for (direction, rotations) in [("SOUTH", 1), ("WEST", 2), ("NORTH", 3)] {
        let destination_dir =
            generated_sprite_dir(out_dir, &format!("sources_{}", direction.to_lowercase()));
        let files = rotated_files(&source_dir, &destination_dir, rotations);
        res.push_str(&include_bytes_slice(
            &format!("LASER_SOURCE_{direction}_BYTES"),
            &files,
        ));
        res.push_str(&include_bytes_const(
            &format!("LASER_SOURCE_{direction}_FALLBACK_BYTES"),
            &rotated_fallback_file(&source_dir, &destination_dir, rotations),
        ));
    }

    res
}

fn include_sprites_in_binary() {
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let cwd = env::current_dir().unwrap();
    // Append '/' to cwd
    let cwd = format!("{}/", cwd.to_str().unwrap());

    let mut res = agent_files();
    res.push_str(&laser_files(&out_dir));
    res.push_str(&laser_source_files(&out_dir));
    res.push_str(&format!(
        "pub const GEM_BYTES: &[u8] = include_bytes!(r#\"{cwd}/{RESOURCES}/gem.png\"#);\n",
    ));
    res.push_str(&format!(
        "pub const VOID_BYTES: &[u8] = include_bytes!(r#\"{cwd}/{RESOURCES}/void.png\"#);\n",
    ));
    let dest_path = out_dir.join("constants.rs");
    fs::write(dest_path, res).unwrap();
}

fn _make_readme() {
    let readme = fs::read_to_string("docs/readme_pypi.md").unwrap();
    let mut readme = readme.replace("lvl6-annotated.png", "docs/lvl6-annotated.png");
    readme.push_str(&fs::read_to_string("docs/readme_build.md").unwrap());
    fs::write("readme.md", readme).unwrap();
}

fn main() {
    include_sprites_in_binary();
    // make_readme();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={RESOURCES}/agents");
    println!("cargo:rerun-if-changed={RESOURCES}/lasers");
    println!("cargo:rerun-if-changed={RESOURCES}/sources");
    println!("cargo:rerun-if-changed={RESOURCES}/gem.png");
    println!("cargo:rerun-if-changed={RESOURCES}/void.png");
}
