//! Each page's name, said as its title and its heading, and as a link to
//! it where one names it. A button's words stay the button's.

/// The sign-in (views/login.rs).
pub const SIGN_IN: &str = "Sign in";

/// The sign-in's code step (views/login.rs).
pub const SIGN_IN_CODE: &str = "Your sign-in code";

/// Signing up (views/register.rs).
pub const SIGN_UP: &str = "Sign up";

/// The forgotten password's form (views/forgot.rs), and the sign-in's link
/// to it.
pub const FORGOT_PASSWORD: &str = "Forgot your password?";

/// The page the welcome mail's link opens (views/verify.rs).
pub const VERIFY_EMAIL: &str = "Verify your email";

/// The account page (views/account.rs), and the signed-in page's link to
/// it.
pub const ACCOUNT: &str = "Your account";

/// The page not found (views/not_found.rs).
pub const NOT_FOUND: &str = "Page not found";

/// The 429 page (views/too_many_requests.rs).
pub const TOO_MANY_REQUESTS: &str = "Too many requests";

/// The template's parts, for the browser tests (views/parts.rs).
pub const PARTS: &str = "Parts";
