use clap::ArgMatches;
use std::{
    fs::File,
    path::{Path, PathBuf},
};
use walkdir::WalkDir;

/// Used to store file or folder and their destination in vpk (zip archive)
struct AddList {
    src: PathBuf,
    dst: String,
}

/// [std::fmt::Debug] implementation for [AddList]
impl std::fmt::Debug for AddList {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AddList")
            .field("src", &self.src)
            .field("dst", &self.dst)
            .finish()
    }
}

/// Default VPK filename
const DEFAULT_OUTPUT_FILE: &str = "output.vpk";
/// Default SFO path in the VPK file
const DEFAULT_SFO_VPK_PATH: &str = "sce_sys/param.sfo";
/// Default EBOOT path in the VPK file
const DEFAULT_EBOOT_VPK_PATH: &str = "eboot.bin";

/// Main function of vita-pack-vpk. Parse all the command line options and
/// arguments.
fn main() {
    use clap::{App, Arg};

    let addlist_vec: Vec<AddList>;
    let vpk_path: &Path;
    let arg_matches: ArgMatches = App::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .arg(
            Arg::new("sfo")
                .short('s')
                .long("sfo")
                .value_name("param.sfo")
                .about("Sets the param.sfo file")
                .validator(check_file)
                .takes_value(true)
                .required(true)
                .display_order(1),
        )
        .arg(
            Arg::new("eboot")
                .short('b')
                .long("eboot")
                .value_name("eboot.bin")
                .about("Sets the eboot.bin file")
                .validator(check_file)
                .takes_value(true)
                .required(true)
                .display_order(2),
        )
        .arg(
            Arg::new("add")
                .short('a')
                .long("add")
                .value_name("src=dst")
                .about("Adds the file or directory src to the vpk as dst")
                .validator(check_add)
                .multiple_occurrences(true)
                .display_order(3),
        )
        .arg(
            Arg::new("vpk")
                .about("Name and path to the new .vpk file")
                .required(false)
                .default_value(DEFAULT_OUTPUT_FILE),
        )
        .get_matches();
    addlist_vec = build_list(&arg_matches);
    vpk_path = Path::new(arg_matches.value_of("vpk").unwrap_or_default());
    match pack_vpk(addlist_vec, vpk_path) {
        Ok(file) => println!("File successfully created [{:?}]", file),
        Err(error) => println!("Error: {}", error),
    }
}

/// Function used to check if path are files and exists for Clap
fn check_file(file: &str) -> Result<(), String> {
    let file_path = Path::new(&file);
    if !file_path.exists() {
        Err(String::from("File doesn't exist!"))
    } else if !file_path.is_file() {
        Err(String::from("Given path is not a valid file!"))
    } else {
        Ok(())
    }
}

/// Function used to check if "--add" option are correct for Clap
fn check_add(var: &str) -> Result<(), String> {
    if var.contains("=") && var.len() >= 3 {
        Ok(())
    } else {
        Err(String::from("Need <src=dst>. With src the source folder or path and dst where it should be in the vpk archive."))
    }
}

/// Function that will build an [Vec]<[AddList]>. That will set the parsed
/// options: sfo, eboot and add(s)
///
/// Returns an [Vec]<[AddList]>
fn build_list(arg_matches: &ArgMatches) -> Vec<AddList> {
    let sfo_path: &Path;
    let eboot_path: &Path;
    let mut addlist_vec: Vec<AddList>;

    // Get sfo and eboot path from [Clap] arguments matches
    sfo_path = Path::new(arg_matches.value_of("sfo").unwrap());
    eboot_path = Path::new(arg_matches.value_of("eboot").unwrap());

    // Create our addlist Vector and push sfo and eboot addlists
    addlist_vec = Vec::new();
    addlist_vec.push(make_add_list(sfo_path, String::from(DEFAULT_SFO_VPK_PATH)));
    addlist_vec.push(make_add_list(
        eboot_path,
        String::from(DEFAULT_EBOOT_VPK_PATH),
    ));

    // Check if add options are present, parse them, create and addlist and add
    // them to the AddList Vector
    if arg_matches.is_present("add") {
        for entry in arg_matches.values_of("add").unwrap() {
            let path = Path::new(entry);
            if path.is_file() {
                addlist_vec.push(parse_add(entry));
            }
            if path.is_dir() {
                addlist_vec.append(&mut walk_list(parse_add(entry)));
            }
        }
    }

    addlist_vec
}

fn walk_list(addlist: AddList) -> Vec<AddList> {
    let mut addlist_vec: Vec<AddList>;

    for entry in WalkDir::new(addlist.src).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.strip_prefix(Path::new(addlist.dst)).unwrap();
    }

    addlist_vec
}

/// Function that will make an [AddList] struct from the source [Path] and
/// destination path as [String]
///
/// Returns a fully completed [AddList].
///
/// ### Example
/// ```rust
/// let src_path = Path::new("/home/usernumberone/sourcefolder/sourcefile.src");
/// let dst_path = String::from("/sourcefolder/sourcefile.src");
/// let addlist = make_add_list(src_path, dst_path);
/// println!("{:?}", addlist);
/// ```
fn make_add_list(src_path: &Path, dst_path: String) -> AddList {
    if !src_path.exists() {
        println!(
            "[ERR] Given file or folder doesn't exists: {}",
            src_path.to_str().unwrap()
        );
        std::process::exit(exitcode::NOINPUT);
    }
    AddList {
        src: src_path.to_path_buf(),
        dst: dst_path,
    }
}

/// Function that will split and collect one of parsed "--add" options into two
/// element, one source [Path] and one destination path as [String]
///
/// Returns a fully completed [AddList] using [vita-pack_vpk]::[make_add_list]
///
/// ### Example
/// ```rust
/// let arg_add = "source/folder=destination";
/// let addlist = parse_add(arg_add);
/// println!("{:?}", addlist);
/// ```
fn parse_add(arg_add: &str) -> AddList {
    let splitted_arg_add: Vec<&str> = arg_add.split("=").collect();
    let src_path: &Path = Path::new(splitted_arg_add[0]);
    let dst_str: String = String::from(splitted_arg_add[1]);

    make_add_list(src_path, dst_str)
}

/// Function that will make a file and verify that it's have benne correctly
/// created
///
/// Returns a [File] on success.
///
/// ### Example
/// ```rust
/// let file_path = Path::new("path/to/the/file.dst");
/// let file_file = make_file(file_path);
/// println!("{:?}", file_file)
/// ```
fn make_file(file_path: &Path) -> File {
    match File::create(file_path) {
        Ok(file) => file,
        Err(error) => {
            println!(
                "error: Unable to make the {:?} file : {:?}",
                file_path.to_str(),
                error
            );
            std::process::exit(exitcode::CANTCREAT);
        }
    }
}

/// Function that will make the VPK archive with all the required files
///
/// This is the final step of vita-pack-vpk. It returns nothing.
fn pack_vpk(addlist: Vec<AddList>, vpk_path: &Path) -> zip::result::ZipResult<()> {
    use std::io::prelude::*;
    use zip::{write::FileOptions, CompressionMethod::Stored, ZipWriter};

    // Variable that will manage ZipWriter (Zip Archive Generator) to write our
    // vpk file
    let mut vpk_writer: ZipWriter<File>;
    let mut file_buff = Vec::new();
    let options: FileOptions;
    // Variable that will allow to create and write to our new vpk file
    let vpk_file: File;

    vpk_file = make_file(vpk_path);
    vpk_writer = ZipWriter::new(vpk_file);
    options = FileOptions::default()
        .compression_method(Stored)
        .unix_permissions(0o755);

    for pair in addlist {
        let mut file = File::open(&pair.src)?;
        file.read_to_end(&mut file_buff)?;
        vpk_writer.start_file(&pair.dst, options)?;
        vpk_writer.write_all(&*file_buff)?;
        file_buff.clear();
    }
    vpk_writer.finish()?;
    Ok(())
}