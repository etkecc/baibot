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
