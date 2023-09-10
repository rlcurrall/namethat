use askama::Template;

#[derive(Template)]
#[template(path = "pages/index.jinja")]
pub struct Index {
    authenticated: bool,
}

impl Index {
    pub fn new(authenticated: bool) -> Self {
        Self { authenticated }
    }
}

#[derive(Template)]
#[template(path = "pages/auth/login.jinja")]
pub struct Login {}

impl Login {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Template)]
#[template(path = "pages/auth/register.jinja")]
pub struct Register {}

impl Register {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Template)]
#[template(path = "pages/profile.jinja")]
pub struct Profile {
    authenticated: bool,
}

impl Profile {
    pub fn new() -> Self {
        Self {
            authenticated: true,
        }
    }
}

#[derive(Template)]
#[template(path = "pages/error/401.jinja")]
pub struct Unauthorized {
    authenticated: bool,
}

impl Unauthorized {
    pub fn new(authenticated: bool) -> Self {
        Self { authenticated }
    }
}

#[derive(Template)]
#[template(path = "pages/error/403.jinja")]
pub struct Forbidden {
    authenticated: bool,
}

impl Forbidden {
    pub fn new(authenticated: bool) -> Self {
        Self { authenticated }
    }
}

#[derive(Template)]
#[template(path = "pages/error/404.jinja")]
pub struct NotFound {
    authenticated: bool,
}

impl NotFound {
    pub fn new(authenticated: bool) -> Self {
        Self { authenticated }
    }
}

#[derive(Template)]
#[template(path = "pages/error/500.jinja")]
pub struct ServerError {
    authenticated: bool,
}

impl ServerError {
    pub fn new(authenticated: bool) -> Self {
        Self { authenticated }
    }
}

#[derive(Template)]
#[template(path = "pages/error/503.jinja")]
pub struct ServiceUnavailable {
    authenticated: bool,
}

impl ServiceUnavailable {
    pub fn new(authenticated: bool) -> Self {
        Self { authenticated }
    }
}

#[derive(Template)]
#[template(path = "pages/games/index.jinja")]
pub struct Games {
    authenticated: bool,
}

impl Games {
    pub fn new() -> Self {
        Self {
            authenticated: true,
        }
    }
}

#[derive(Template)]
#[template(path = "pages/games/create.jinja")]
pub struct CreateGame {
    authenticated: bool,
}

impl CreateGame {
    pub fn new() -> Self {
        Self {
            authenticated: true,
        }
    }
}

#[derive(Template)]
#[template(path = "pages/games/play.jinja")]
pub struct PlayGame {
    authenticated: bool,
}

impl PlayGame {
    pub fn new(authenticated: bool) -> Self {
        Self { authenticated }
    }
}

#[derive(Template)]
#[template(path = "pages/games/run.jinja")]
pub struct RunGame {
    authenticated: bool,
}

impl RunGame {
    pub fn new(authenticated: bool) -> Self {
        Self { authenticated }
    }
}

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! impl_to_html {
        ($($view:ident),*) => {
            $(
                impl $view {
                    pub fn to_html(&self) -> crate::error::AppResult<axum::response::Html<String>> {
                        let html = self
                            .render()
                            .map_err(|e| crate::error::AppError::InternalError(e.to_string()))?;
                        Ok(axum::response::Html(html))
                    }
                }
            )*
        };
    }
}

impl_to_html!(
    Index,
    Login,
    Register,
    Profile,
    Unauthorized,
    Forbidden,
    NotFound,
    ServerError,
    ServiceUnavailable,
    Games,
    CreateGame,
    PlayGame,
    RunGame
);
