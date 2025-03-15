use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub struct MimeType {
    main_type: MainMimeType,
    sub_type: SubMimeType,
    original: String,
}

#[derive(Debug, PartialEq)]
pub enum MainMimeType {
    Application,
    Audio,
    Font,
    Image,
    Text,
    Video,
}

#[derive(Debug, PartialEq)]
pub enum SubMimeType {
    AAC,
    ABW,
    APNG,
    ARC,
    AVIF,
    AVI,
    AZW,
    BIN,
    BMP,
    BZ,
    BZ2,
    CDA,
    CSH,
    CSS,
    CSV,
    DOC,
    DOCX,
    EOT,
    EPUB,
    GZ,
    GIF,
    HTM,
    ICO,
    ICS,
    JAR,
    JPEG,
    JS,
    JSON,
    JSONLD,
    MID,
    MJS,
    MP3,
    MP4,
    MPEG,
    MPKG,
    ODP,
    ODS,
    ODT,
    // Could be Opus-encoded
    OGA,
    OGV,
    OGX,
    OTF,
    PNG,
    PDF,
    PHP,
    PPT,
    PPTX,
    RAR,
    RTF,
    SH,
    SVG,
    TAR,
    TIF,
    TS,
    TTF,
    TXT,
    VSD,
    WAV,
    WEBA,
    WEBM,
    WEBP,
    WOFF,
    WOFF2,
    XHTML,
    XLS,
    XLSX,
    XML,
    XUL,
    ZIP,
    _3GP,
    _3G2,
    _7Z,
}

impl FromStr for MimeType {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (main_type, sub_type) = match s {
            "audio/aac" => (MainMimeType::Audio, SubMimeType::AAC),
            "application/x-abiword" => (MainMimeType::Application, SubMimeType::ABW),
            "image/apng" => (MainMimeType::Image, SubMimeType::APNG),
            "application/x-freearc" => (MainMimeType::Application, SubMimeType::ARC),
            "image/avif" => (MainMimeType::Image, SubMimeType::AVIF),
            "video/x-msvideo" => (MainMimeType::Video, SubMimeType::AVI),
            "application/vnd.amazon.ebook" => (MainMimeType::Application, SubMimeType::AZW),
            "application/octet-stream" => (MainMimeType::Application, SubMimeType::BIN),
            "image/bmp" => (MainMimeType::Image, SubMimeType::BMP),
            "application/x-bzip" => (MainMimeType::Application, SubMimeType::BZ),
            "application/x-bzip2" => (MainMimeType::Application, SubMimeType::BZ2),
            "application/x-cdf" => (MainMimeType::Application, SubMimeType::CDA),
            "application/x-csh" => (MainMimeType::Application, SubMimeType::CSH),
            "text/css" => (MainMimeType::Text, SubMimeType::CSS),
            "text/csv" => (MainMimeType::Text, SubMimeType::CSV),
            "application/msword" => (MainMimeType::Application, SubMimeType::DOC),
            "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
                (MainMimeType::Application, SubMimeType::DOCX)
            }
            "application/vnd.ms-fontobject" => (MainMimeType::Application, SubMimeType::EOT),
            "application/epub+zip" => (MainMimeType::Application, SubMimeType::EPUB),
            "application/gzip" | ".gz" | "application/x-gzip" => {
                (MainMimeType::Application, SubMimeType::GZ)
            }
            "image/gif" => (MainMimeType::Image, SubMimeType::GIF),
            "text/html" => (MainMimeType::Text, SubMimeType::HTM),
            "image/vnd.microsoft.icon" => (MainMimeType::Image, SubMimeType::ICO),
            "text/calendar" => (MainMimeType::Text, SubMimeType::ICS),
            "application/java-archive" => (MainMimeType::Application, SubMimeType::JAR),
            "image/jpeg" => (MainMimeType::Image, SubMimeType::JPEG),
            "text/javascript" => (MainMimeType::Text, SubMimeType::JS),
            "application/json" => (MainMimeType::Application, SubMimeType::JSON),
            "application/ld+json" => (MainMimeType::Application, SubMimeType::JSONLD),
            "audio/midi" | "audio/x-midi" => (MainMimeType::Audio, SubMimeType::MID),
            "audio/mpeg" => (MainMimeType::Audio, SubMimeType::MP3),
            "video/mp4" => (MainMimeType::Video, SubMimeType::MP4),
            "video/mpeg" => (MainMimeType::Video, SubMimeType::MPEG),
            "application/vnd.apple.installer+xml" => (MainMimeType::Application, SubMimeType::MPKG),
            "application/vnd.oasis.opendocument.presentation" => {
                (MainMimeType::Application, SubMimeType::ODP)
            }
            "application/vnd.oasis.opendocument.spreadsheet" => {
                (MainMimeType::Application, SubMimeType::ODS)
            }
            "application/vnd.oasis.opendocument.text" => {
                (MainMimeType::Application, SubMimeType::ODT)
            }
            "audio/ogg" => (MainMimeType::Audio, SubMimeType::OGA),
            "video/ogg" => (MainMimeType::Video, SubMimeType::OGV),
            "application/ogg" => (MainMimeType::Application, SubMimeType::OGX),
            "font/otf" => (MainMimeType::Font, SubMimeType::OTF),
            "image/png" => (MainMimeType::Image, SubMimeType::PNG),
            "application/pdf" => (MainMimeType::Application, SubMimeType::PDF),
            "application/x-httpd-php" => (MainMimeType::Application, SubMimeType::PHP),
            "application/vnd.ms-powerpoint" => (MainMimeType::Application, SubMimeType::PPT),
            "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
                (MainMimeType::Application, SubMimeType::PPTX)
            }
            "application/vnd.rar" => (MainMimeType::Application, SubMimeType::RAR),
            "application/rtf" => (MainMimeType::Application, SubMimeType::RTF),
            "application/x-sh" => (MainMimeType::Application, SubMimeType::SH),
            "image/svg+xml" => (MainMimeType::Image, SubMimeType::SVG),
            "application/x-tar" => (MainMimeType::Application, SubMimeType::TAR),
            "image/tiff" => (MainMimeType::Image, SubMimeType::TIF),
            "video/mp2t" => (MainMimeType::Video, SubMimeType::TS),
            "font/ttf" => (MainMimeType::Font, SubMimeType::TTF),
            "text/plain" => (MainMimeType::Text, SubMimeType::TXT),
            "application/vnd.visio" => (MainMimeType::Application, SubMimeType::VSD),
            "audio/wav" => (MainMimeType::Audio, SubMimeType::WAV),
            "audio/webm" => (MainMimeType::Audio, SubMimeType::WEBA),
            "video/webm" => (MainMimeType::Video, SubMimeType::WEBM),
            "image/webp" => (MainMimeType::Image, SubMimeType::WEBP),
            "font/woff" => (MainMimeType::Font, SubMimeType::WOFF),
            "font/woff2" => (MainMimeType::Font, SubMimeType::WOFF2),
            "application/xhtml+xml" => (MainMimeType::Application, SubMimeType::XHTML),
            "application/vnd.ms-excel" => (MainMimeType::Application, SubMimeType::XLS),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
                (MainMimeType::Application, SubMimeType::XLSX)
            }
            "application/xml" | "text/xml" | ".xml" | "application/atom+xml" => {
                (MainMimeType::Application, SubMimeType::XML)
            }
            "application/vnd.mozilla.xul+xml" => (MainMimeType::Application, SubMimeType::XUL),
            "application/zip" | ".zip" | "application/x-zip-compressed" => {
                (MainMimeType::Application, SubMimeType::ZIP)
            }
            "video/3gpp" | "audio/3gpp" => (MainMimeType::Video, SubMimeType::_3GP),
            "video/3gpp2" | "audio/3gpp2" => (MainMimeType::Video, SubMimeType::_3G2),
            "application/x-7z-compressed" => (MainMimeType::Application, SubMimeType::_7Z),
            _ => return Err("Not a valid MIME type"),
        };

        Ok(MimeType {
            main_type,
            sub_type,
            original: s.to_string(),
        })
    }
}
