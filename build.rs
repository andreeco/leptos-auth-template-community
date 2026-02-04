use leptos_i18n_build::{Config, FileFormat, ParseOptions, TranslationsInfos};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn Error>> {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-changed=Cargo.toml");

    let i18n_mod_directory = PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("i18n");

    // --- Automatic namespace detection from locales/de/*.toml ---
    let ns_base = PathBuf::from("locales/de");
    let mut namespaces = vec![];
    for entry in fs::read_dir(&ns_base)? {
        let entry = entry?;
        let fname = entry.file_name();
        let fname = fname.to_string_lossy();
        if let Some(stem) = fname.strip_suffix(".toml") {
            namespaces.push(stem.to_string());
        }
    }
    namespaces.sort();
    // println!("cargo:warning=Leptos i18n Namespaces: {:?}", namespaces);

    let options = ParseOptions::default().file_format(FileFormat::Toml);
    let mut cfg = Config::new("de")?.add_locale("en")?.parse_options(options);
    for ns in &namespaces {
        cfg = cfg.add_namespace(ns)?;
    }

    let translations_infos = TranslationsInfos::parse(cfg)?;

    translations_infos.emit_diagnostics();
    translations_infos.rerun_if_locales_changed();
    translations_infos.generate_i18n_module(i18n_mod_directory)?;
    Ok(())
}
