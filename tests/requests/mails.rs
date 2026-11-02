//! Reading what the stub mailer got (`ctx.mailer`'s deliveries): the
//! sender, the subject, the text with its transfer encoding undone, and the
//! HTML part as a mail program renders it.

/// Every auth mail is sent from `settings.mail.from` (`config/test.yaml`),
/// never Loco's default `System <system@example.com>`.
pub fn assert_from_the_configured_sender(mail: &str) {
    let from = mail
        .lines()
        .find(|line| line.starts_with("From: "))
        .unwrap_or_else(|| panic!("no From header:\n{mail}"));
    assert!(
        from.contains("SaaS Starter") && from.contains("<noreply@example.com>"),
        "sender is not settings.mail.from: {from}"
    );
}

/// The `Subject:` line.
pub fn subject(mail: &str) -> &str {
    mail.lines()
        .find(|line| line.starts_with("Subject: "))
        .unwrap_or_else(|| panic!("no Subject header:\n{mail}"))
}

/// The whole message with quoted-printable's soft line breaks and its
/// `=3D` undone, so a link reads as written.
pub fn unfolded(mail: &str) -> String {
    mail.replace("=\r\n", "")
        .replace("=\n", "")
        .replace("=3D", "=")
}

/// The `text/html` part of a raw multipart message, with its transfer
/// encoding undone, so the test reads what a mail client renders.
pub fn html_part(mail: &str) -> String {
    let start = mail
        .find("Content-Type: text/html")
        .unwrap_or_else(|| panic!("no HTML part:\n{mail}"));
    let (head, rest) = mail[start..]
        .split_once("\r\n\r\n")
        .unwrap_or_else(|| panic!("an HTML part without a body:\n{mail}"));
    let body = rest.split("\r\n--").next().unwrap_or(rest);
    if head.contains("quoted-printable") {
        decode_quoted_printable(body)
    } else {
        assert!(
            !head.contains("base64"),
            "the HTML part is base64 now, decode it here:\n{head}"
        );
        body.to_owned()
    }
}

/// Quoted-printable (RFC 2045): soft line breaks removed, `=XX` turned back
/// into its byte.
fn decode_quoted_printable(body: &str) -> String {
    let unfolded = body.replace("=\r\n", "");
    let bytes = unfolded.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'='
            && let Some(byte) = bytes
                .get(i + 1..i + 3)
                .and_then(|hex| std::str::from_utf8(hex).ok())
                .and_then(|hex| u8::from_str_radix(hex, 16).ok())
        {
            decoded.push(byte);
            i += 3;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(decoded).expect("the HTML part is UTF-8")
}
