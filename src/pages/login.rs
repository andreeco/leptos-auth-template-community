// use crate::i18n::*;
use leptos::prelude::*;
// use leptos_meta::{Link as MetaLink, Title};
#[cfg(feature = "ssr")]
use http::StatusCode;

#[server(LoginUser)]
pub async fn login_user(username: String, password: String) -> Result<(), ServerFnError> {
    use crate::auth::{AuthSession, Credentials};

    let res = leptos_axum::ResponseOptions::default();
    //let Extension(mut auth): Extension<AuthSession> = leptos_axum::extract().await?;

    let mut auth: AuthSession = expect_context::<AuthSession>();
    let creds = Credentials { username, password };

    match auth.authenticate(creds).await {
        Ok(Some(user)) => {
            if auth.login(&user).await.is_err() {
                res.set_status(StatusCode::INTERNAL_SERVER_ERROR);
                return Err(ServerFnError::ServerError("Login failed".into()));
            }
            // provide context
            provide_context(auth.clone());
            leptos_axum::redirect("/protectedleptos");
            Ok(())
        }
        Ok(None) => {
            res.set_status(http::StatusCode::UNAUTHORIZED);
            Err(ServerFnError::ServerError("Invalid credentials".into()))
        }
        Err(e) => {
            res.set_status(http::StatusCode::INTERNAL_SERVER_ERROR);
            Err(ServerFnError::ServerError(e.to_string()))
        }
    }
}

#[component]
pub fn LoginPage() -> impl IntoView {
    let login_action = ServerAction::<LoginUser>::new();

    view! {
        <h1>"Login"</h1>
        {move || login_action.value().get().and_then(|res| res.err()).map(|err| view!{
            <div class="error">{err.to_string()}</div>
        })}
        <ActionForm action=login_action>
            <input type="text" name="username" placeholder="username" value="user"/>
            <input type="password" name="password" placeholder="password" value="password"/>
            <button type="submit">"Log In"</button>
        </ActionForm>
    }
}
