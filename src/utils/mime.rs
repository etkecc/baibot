use mxlink::mime;

pub fn get_file_extension(mime_type: &mime::Mime) -> String {
    match (mime_type.type_(), mime_type.subtype()) {
        (mime::AUDIO, mime::BASIC) => "au",
        (mime::AUDIO, mime::MPEG) => "mp3",
        (mime::AUDIO, mime::MP4) => "m4a",
        (mime::AUDIO, mime::OGG) => "ogg",
        (mime::IMAGE, mime::BMP) => "bmp",
        (mime::IMAGE, mime::GIF) => "gif",
        (mime::IMAGE, mime::JPEG) => "jpg",
        (mime::IMAGE, mime::PNG) => "png",
        (mime::IMAGE, mime::SVG) => "svg",
        _ => "bin",
    }
    .to_string()
}

pub fn get_mime_type_from_file_name(file_name: &str) -> mime::Mime {
    let extension = file_name.rsplit('.').next().unwrap_or("");

    match extension.to_lowercase().as_str() {
        "jpg" | "jpeg" => mime::IMAGE_JPEG,
        "png" => mime::IMAGE_PNG,
        "gif" => mime::IMAGE_GIF,
        "webp" => "image/webp".parse().unwrap(),
        "svg" => mime::IMAGE_SVG,
        "tiff" | "tif" => "image/tiff".parse().unwrap(),
        "bmp" => "image/bmp".parse().unwrap(),
        "heic" | "heif" => "image/heic".parse().unwrap(),
        "avif" => "image/avif".parse().unwrap(),
        _ => mime::APPLICATION_OCTET_STREAM,
    }
}
