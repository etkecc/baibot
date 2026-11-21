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
        // No constant for it in the `mime` crate.
        (mime::IMAGE, subtype) if subtype == "webp" => "webp",
        _ => "bin",
    }
    .to_string()
}

pub fn get_mime_type_from_file_name(file_name: &str) -> mime::Mime {
    mime_guess::from_path(file_name)
        .first()
        .unwrap_or(mime::APPLICATION_OCTET_STREAM)
}

#[cfg(test)]
mod tests {
    use super::{get_file_extension, get_mime_type_from_file_name};

    #[test]
    fn image_types_survive_a_round_trip_through_file_names() {
        // The bot names the images it posts after their type, and reads the type back from the name.
        for mime_type in ["image/png", "image/jpeg", "image/gif", "image/webp"] {
            let mime_type: mxlink::mime::Mime = mime_type.parse().unwrap();
            let file_name = format!("generated-image.{}", get_file_extension(&mime_type));

            assert_eq!(get_mime_type_from_file_name(&file_name), mime_type);
        }
    }
}
