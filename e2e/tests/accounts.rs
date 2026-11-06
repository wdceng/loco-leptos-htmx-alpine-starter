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
