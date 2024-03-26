use std::time::Duration;
use uuid::Uuid;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get,put},
    Json, Router,
  };
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tokio::net::TcpListener;


#[tokio::main]
async fn main() {
    //expose environment variables from .env file
    dotenvy::dotenv().expect("Unable to access .env file");
      //set variables from enviroment variables
    let server_address = std::env::var("SERVER_ADDRESS").unwrap_or("127.0.0.1:3000".to_owned());
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not found in env file");
    
    //create our database pool
    let db_pool = PgPoolOptions::new()
        .max_connections(64)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&database_url)
        .await
        .expect("can't connect to database");

    //create our tcp listener
    let listener = TcpListener::bind(server_address)
        .await
        .expect("Could not create tcp listener");

    println!("listening on {}", listener.local_addr().unwrap());
    // compose the routes
    let app = Router::new()
        .route("/", get(|| async { "Hello world" }))
        .route("/posts", get(get_posts).post(create_post))
        .route("/posts/:post_id", put(update_post).delete(delete_post))
        .with_state(db_pool);
    //serve the application
    axum::serve(listener, app)
        .await
        .expect("Error serving application");
}
async fn get_posts(
    State(db_pool): State<PgPool>,
  ) -> Result<(StatusCode, String), (StatusCode, String)> {
    let rows = sqlx::query_as!(PostRow, "SELECT * FROM posts")
      .fetch_all(&db_pool)
      .await
      .map_err(|e| {
        (
          StatusCode::INTERNAL_SERVER_ERROR,
          json!({"success": false, "message": e.to_string()}).to_string(),
        )
      })?;
  
    Ok((
      StatusCode::OK,
      json!({"success": true, "data": rows}).to_string(),
    ))
  }
  async fn create_post(
    State(db_pool): State<PgPool>,
    Json(post): Json<CreatePostSchema>,
  ) -> Result<(StatusCode, String), (StatusCode, String)> {
    let row = sqlx::query_as!(
        PostRow,
      "INSERT INTO posts (message, username, day) VALUES ($1, $2, $3) RETURNING *",
      post.message,
      post.username,
      post.day
    )
    .fetch_one(&db_pool)
    .await
    .map_err(|e| {
      (
        StatusCode::INTERNAL_SERVER_ERROR,
        json!({"success": false, "message": e.to_string()}).to_string(),
      )
    })?;
  
    Ok((
      StatusCode::CREATED,
      json!({"success": true, "data": row}).to_string(),
    ))
  }

async fn update_post(  State(db_pool): State<PgPool>,
Path(post_id): Path<Uuid>,
Json(post): Json<UpdatePostSchema>,)
 -> Result<(StatusCode, String), (StatusCode, String)> {

    let mut query = "UPDATE posts SET id = $1".to_owned();

    let mut i = 2;


  if post.message.is_some() {
    query.push_str(&format!(", message = ${i}"));
    i = i + 1;
  };

  if post.username.is_some() {
    query.push_str(&format!(", username = ${i}"));
    i=i+1;
  };

  if post.day.is_some() {
    query.push_str(&format!(", day = ${i}"));
  };

  query.push_str(&format!(" WHERE id = $1"));

  let mut s = sqlx::query(&query).bind(post_id);

  if post.message.is_some() {
    s = s.bind(post.message);
  }

  if post.username.is_some() {
    s = s.bind(post.username);
  }
  if post.day.is_some() {
    s = s.bind(post.day);
  }

  s.execute(&db_pool).await.map_err(|e| {
    (
      StatusCode::INTERNAL_SERVER_ERROR,
      json!({"success": false, "message": e.to_string()}).to_string(),
    )
  })?;

    Ok((StatusCode::OK, json!({"success":true}).to_string()))
 }

  async fn delete_post(
    State(db_pool): State<PgPool>,
    Path(post_id): Path<Uuid>,
  ) -> Result<(StatusCode, String), (StatusCode, String)> {
    sqlx::query!("DELETE FROM posts WHERE id = $1", post_id,)
      .execute(&db_pool)
      .await
      .map_err(|e| {
        (
          StatusCode::INTERNAL_SERVER_ERROR,
          json!({"success": false, "message": e.to_string()}).to_string(),
        )
      })?;
  
    Ok((StatusCode::OK, json!({"success":true}).to_string()))
  }
 

  #[derive(Deserialize)]
  pub struct CreatePostSchema {
      pub message: String,
      pub username: String,
      pub day: String,
  }
  

#[derive(Serialize)]
struct PostRow {
    id: Uuid,
    message: String,
    username: String,
    day: String,
    #[serde(rename = "createdAt")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(rename = "updatedAt")]
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}


#[derive(Deserialize)]
pub struct UpdatePostSchema {
    pub message: Option<String>,
    pub username: Option<String>,
    pub day: Option<String>
}