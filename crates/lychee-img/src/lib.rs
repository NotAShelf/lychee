use std::path::Path;

pub fn is_image_file(path: &Path) -> bool {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    matches!(
        ext.to_lowercase().as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "gif"
            | "bmp"
            | "tiff"
            | "tif"
            | "webp"
            | "ico"
            | "ppm"
            | "pgm"
            | "pbm"
            | "pn"
            | "svg"
            | "tga"
    )
}

pub fn is_hidden_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .map(|n| n.starts_with('.'))
        .unwrap_or(false)
}

pub fn collect_paths(paths: Vec<String>) -> Vec<String> {
    if paths.len() == 1 {
        let p = Path::new(&paths[0]);
        if p.is_dir() {
            let mut result: Vec<String> = std::fs::read_dir(p)
                .ok()
                .map(|dir| {
                    dir.filter_map(|e| {
                        let path = e.ok()?.path();
                        if is_image_file(&path) && !is_hidden_file(&path) {
                            Some(path.to_string_lossy().into_owned())
                        } else {
                            None
                        }
                    })
                    .collect()
                })
                .unwrap_or_default();
            result.sort();
            result
        } else {
            vec![paths[0].clone()]
        }
    } else {
        paths
    }
}
