//! The accounts in a browser, as a visitor goes through them: signing up
//! with the welcome mail's link, signing in in place, a wrong password
//! refused, an admin's mailed code, and the account page's dialog and
//! photo. The request tests prove what the server answers
//! (tests/requests/); these, what the page does with it under htmx and
//! Alpine. Each test signs up an account of its own ([`mailbox`]), its
//! mails read from the lane's Mailpit.

use app_e2e::{PASSWORD, browse, mailbox};
use serde_json::Value;

/// A visitor signs up, verifies through the mail's link, and signs in: the
/// form sent in place (the shell's boost), htmx following the answer's
/// redirect, and the session's cookie, set in htmx's answer, signing the
/// next page in.
#[tokio::test]
async fn a_visitor_signs_up_and_signs_in_in_place() {
    browse(|page| async move {
        let email = mailbox("visitor");
        page.sign_up("Ana", &email).await?;
        page.open("/login").await?;
        page.mark().await?;
        page.type_into("#email", &email).await?;
        page.type_into("#password", PASSWORD).await?;
        page.press("form[action='/login'] button[type=submit]")
            .await?;
        page.until("the home page", "return location.pathname === '/'")
            .await?;
        page.ready().await?;
        assert!(page.in_place().await?, "signed in without a page load");
        page.open("/account").await?;
        assert!(
            page.says(&email).await?,
            "the account page, signed in as the new account"
        );
        Ok(())
    })
    .await;
}

/// A wrong password is said in the region: "Error: " first in the title,
/// the field marked and the focus back in it for the next try.
#[tokio::test]
async fn a_wrong_password_is_refused_in_the_region() {
    browse(|page| async move {
        let email = mailbox("refused");
        // The refusal's 400, which Chrome logs.
        page.allow_failed_load(400, "/login");
        page.sign_up("Ana", &email).await?;
        page.open("/login").await?;
        page.mark().await?;
        page.type_into("#email", &email).await?;
        page.type_into("#password", "not the password").await?;
        page.press("form[action='/login'] button[type=submit]")
            .await?;
        page.until(
            "the refusal's title",
            "return document.title.startsWith('Error: ')",
        )
        .await?;
        assert!(page.in_place().await?, "refused in place");
        assert_eq!(
            page.eval("return document.getElementById('password').getAttribute('aria-invalid')")
                .await?,
            Value::from("true"),
            "the password field marked"
        );
        page.until(
            "the focus back in the password",
            "return document.activeElement?.id === 'password'",
        )
        .await?;
        Ok(())
    })
    .await;
}

/// An admin's sign-in asks for the code mailed after the password: the
/// code's page, the code read from the mail and typed, then signed in.
#[tokio::test]
async fn an_admin_signs_in_with_the_mailed_code() {
    browse(|page| async move {
        let email = mailbox("admin");
        page.sign_up("Ana", &email).await?;
        page.make_admin(&email).await?;
        page.sign_in(&email, PASSWORD).await?;
        assert_eq!(page.path().await?, "/login/code", "the code asked for");
        let mail = page.mail_text(&email, "Your sign-in code").await?;
        let code: String = mail
            .split("Your sign-in code: ")
            .nth(1)
            .map(|rest| rest.chars().take(6).collect())
            .unwrap_or_default();
        assert_eq!(code.len(), 6, "a code in the mail:\n{mail}");
        page.type_into("#code", &code).await?;
        page.press("form[action='/login/code'] button[type=submit]")
            .await?;
        page.until("the home page", "return location.pathname === '/'")
            .await?;
        page.open("/account").await?;
        assert!(page.says(&email).await?, "signed in as the admin");
        Ok(())
    })
    .await;
}

/// The account's deletion is asked in a dialog: the focus in it, on Cancel
/// where no password is asked (signed in a moment ago), and Cancel closes
/// it with the account kept.
#[tokio::test]
async fn the_accounts_deletion_is_asked_in_a_dialog() {
    browse(|page| async move {
        let email = mailbox("dialog");
        page.sign_up("Ana", &email).await?;
        page.sign_in(&email, PASSWORD).await?;
        page.open("/account").await?;
        page.press("button[popovertarget=delete-account]").await?;
        page.until(
            "the dialog open",
            "return document.getElementById('delete-account').matches(':popover-open')",
        )
        .await?;
        page.until(
            "the focus on Cancel",
            "return document.activeElement?.matches('#delete-account button[popovertargetaction=hide]')",
        )
        .await?;
        page.press("#delete-account button[popovertargetaction=hide]")
            .await?;
        page.until(
            "the dialog closed",
            "return !document.getElementById('delete-account').matches(':popover-open')",
        )
        .await?;
        assert_eq!(page.path().await?, "/account", "still on the account page");
        Ok(())
    })
    .await;
}

/// A large picture chosen for the avatar is shrunk in the browser before it
/// goes (src/islands/shrink.rs): the field then holds a JPEG under the
/// bytes it is brought under, and the server makes the avatar from it.
#[tokio::test]
async fn a_chosen_photo_is_shrunk_before_it_goes() {
    browse(|page| async move {
        let email = mailbox("photo");
        page.sign_up("Ana", &email).await?;
        page.sign_in(&email, PASSWORD).await?;
        page.open("/account").await?;
        // A drawn picture of noise, 2400 by 1600, a PNG of several MB, put
        // in the field as a person's choice is, and its change sent.
        page.eval(
            "const canvas = document.createElement('canvas');
             canvas.width = 2400; canvas.height = 1600;
             const drawing = canvas.getContext('2d');
             const noise = drawing.createImageData(2400, 1600);
             for (let i = 0; i < noise.data.length; i++) {
               noise.data[i] = i % 4 === 3 ? 255 : (Math.random() * 256) | 0;
             }
             drawing.putImageData(noise, 0, 0);
             canvas.toBlob((blob) => {
               const chosen = new DataTransfer();
               chosen.items.add(new File([blob], 'big.png', { type: 'image/png' }));
               const field = document.getElementById('avatar');
               field.files = chosen.files;
               field.dispatchEvent(new Event('change', { bubbles: true }));
             }, 'image/png');",
        )
        .await?;
        page.until(
            "the photo shrunk to a JPEG under 1 MB",
            "const file = document.getElementById('avatar').files[0];
             return !!file && file.name === 'big.jpg' && file.type === 'image/jpeg'
                 && file.size < 1000000",
        )
        .await?;
        page.press("form[action='/account/avatar'] button[type=submit]")
            .await?;
        page.until(
            "the avatar shown",
            "return !!document.querySelector('img[src^=\"/uploads/avatars/\"]')",
        )
        .await?;
        Ok(())
    })
    .await;
}

/// A chosen picture the shrink can't better goes as it was, and of two
/// chosen one after the other the last goes (src/islands/shrink.rs): a
/// JPEG already within its side and bytes is left as it is, a file the
/// browser can't read as a picture (an iPhone's HEIC in Chrome) stays the
/// original for the server to judge, and an earlier run still working puts
/// nothing in the field. Each time the form's buttons are out of use while
/// it works, and back in use once it's done.
#[tokio::test]
async fn a_picture_the_shrink_cant_better_goes_as_it_was_and_the_last_chosen_goes() {
    browse(|page| async move {
        let email = mailbox("pictures");
        page.sign_up("Ana", &email).await?;
        page.sign_in(&email, PASSWORD).await?;
        page.open("/account").await?;
        // Each time the buttons go out of use and back, counted; a picture
        // put in the field as a person's choice is, its change sent.
        page.eval(
            "window.__held = 0; window.__freed = 0;
             const button = document.querySelector(\"form[action='/account/avatar'] button[type=submit]\");
             // A change of state, not of the attribute: a second picture
             // sets `disabled` again on a button already out of use.
             let held = button.disabled;
             new MutationObserver(() => {
                 if (button.disabled === held) return;
                 held = button.disabled;
                 if (held) window.__held++; else window.__freed++;
             }).observe(button, { attributes: true, attributeFilter: ['disabled'] });
             window.__choose = (...files) => {
                 const field = document.getElementById('avatar');
                 for (const file of files) {
                     const chosen = new DataTransfer();
                     chosen.items.add(file);
                     field.files = chosen.files;
                     field.dispatchEvent(new Event('change', { bubbles: true }));
                 }
             };
             window.__picture = (width, height, type) => new Promise((done) => {
                 const canvas = document.createElement('canvas');
                 canvas.width = width; canvas.height = height;
                 const drawing = canvas.getContext('2d');
                 const noise = drawing.createImageData(width, height);
                 for (let i = 0; i < noise.data.length; i++) {
                     noise.data[i] = i % 4 === 3 ? 255 : (Math.random() * 256) | 0;
                 }
                 drawing.putImageData(noise, 0, 0);
                 canvas.toBlob(done, type, 0.8);
             })",
        )
        .await?;
        let field = "document.getElementById('avatar').files[0]";
        let button = "document.querySelector(\"form[action='/account/avatar'] button[type=submit]\")";
        let done = |freed: u32| {
            format!("return window.__held === {freed} && window.__freed === {freed} && !{button}.disabled")
        };

        // A small JPEG, left as it is.
        page.eval(
            "window.__picture(400, 300, 'image/jpeg').then((blob) => {
                 window.__small = blob.size;
                 window.__choose(new File([blob], 'small.jpg', { type: 'image/jpeg' }));
             })",
        )
        .await?;
        page.until("the small JPEG looked at", &done(1)).await?;
        assert_eq!(
            page.eval(&format!(
                "return {field}.name === 'small.jpg' && {field}.size === window.__small"
            ))
            .await?,
            Value::Bool(true),
            "the small JPEG left as it was"
        );

        // No picture the browser can read: the original stays.
        page.eval(
            "window.__choose(new File([new Uint8Array([0, 1, 2, 3])], 'phone.heic', { type: 'image/heic' }))",
        )
        .await?;
        page.until("the unreadable file looked at", &done(2)).await?;
        assert_eq!(
            page.eval(&format!("return {field}.name")).await?,
            Value::from("phone.heic"),
            "the unreadable file left as it was"
        );

        // Two large pictures, one right after the other: the second goes,
        // the buttons back in use once both are done.
        page.eval(
            "Promise.all([window.__picture(2400, 1600, 'image/png'), window.__picture(2400, 1600, 'image/png')])
                 .then(([first, second]) => window.__choose(
                     new File([first], 'first.png', { type: 'image/png' }),
                     new File([second], 'second.png', { type: 'image/png' })))",
        )
        .await?;
        page.until("both pictures shrunk", &done(3)).await?;
        assert_eq!(
            page.eval(&format!("return {field}.name")).await?,
            Value::from("second.jpg"),
            "the last chosen goes"
        );
        Ok(())
    })
    .await;
}

/// A newer build's answer to a sign-in, whose post led on to a page, loads
/// that page plainly: the post's redirect was followed, so a GET goes
/// again, never the post (`newer_build`, src/islands.rs). The made answer
/// keeps the post's address ([`Page::answer_with`]), so the page loaded is
/// `/login`, already signed in.
#[tokio::test]
async fn a_newer_builds_answer_to_a_sign_in_loads_its_page_plainly() {
    browse(|page| async move {
        let email = mailbox("newer");
        page.sign_up("Ana", &email).await?;
        page.open("/login").await?;
        page.mark().await?;
        page.answer_with(
            "/login",
            "const real = await original(...args);
             const headers = new Headers(real.headers);
             headers.set('X-Build', 'e2e-newer');
             const made = new Response(await real.text(), { status: real.status, headers });
             Object.defineProperty(made, 'redirected', { value: real.redirected });
             return made;",
        )
        .await?;
        page.type_into("#email", &email).await?;
        page.type_into("#password", PASSWORD).await?;
        page.press("form[action='/login'] button[type=submit]")
            .await?;
        page.until(
            "a page loaded plainly",
            "return window.__e2eMark !== true && document.readyState === 'complete'",
        )
        .await?;
        page.ready().await?;
        page.open("/account").await?;
        assert_eq!(page.path().await?, "/account", "signed in by the one post");
        Ok(())
    })
    .await;
}
