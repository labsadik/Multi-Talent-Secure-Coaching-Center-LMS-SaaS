use axum::{
    extract::{Path, State},
    Json,
};
use futures_util::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId, DateTime, Document};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{
    auth::AuthUser,
    error::{AppError, AppResult},
    state::AppState,
};

fn default_font_family() -> String { "sans".to_owned() }
fn default_layout_style() -> String { "split".to_owned() }
fn default_visual_effect() -> String { "none".to_owned() }

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSiteRequest {
    pub display_name: String,
    pub headline: String,
    pub tagline: String,
    pub about: String,
    pub theme: String,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_layout_style")]
    pub layout_style: String,
    #[serde(default = "default_visual_effect")]
    pub visual_effect: String,
    pub brand_color: String,
    pub accent_color: String,
    pub logo_url: Option<String>,
    pub hero_image_url: Option<String>,
    pub contact_email: Option<String>,
    pub is_published: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SiteView {
    display_name: String,
    headline: String,
    tagline: String,
    about: String,
    theme: String,
    font_family: String,
    layout_style: String,
    visual_effect: String,
    brand_color: String,
    accent_color: String,
    logo_url: Option<String>,
    hero_image_url: Option<String>,
    contact_email: Option<String>,
    is_published: bool,
}

impl SiteView {
    fn default_for(company_name: &str) -> Self {
        Self {
            display_name: company_name.to_owned(),
            headline: format!("Learn with {company_name}"),
            tagline: "Practical learning, built around you.".to_owned(),
            about: String::new(),
            theme: "ocean".to_owned(),
            font_family: default_font_family(),
            layout_style: default_layout_style(),
            visual_effect: default_visual_effect(),
            brand_color: "#2563eb".to_owned(),
            accent_color: "#14b8a6".to_owned(),
            logo_url: None,
            hero_image_url: None,
            contact_email: None,
            is_published: false,
        }
    }

    fn from_document(document: &Document, company_name: &str) -> Self {
        let default = Self::default_for(company_name);
        Self {
            display_name: document.get_str("displayName").unwrap_or(&default.display_name).to_owned(),
            headline: document.get_str("headline").unwrap_or(&default.headline).to_owned(),
            tagline: document.get_str("tagline").unwrap_or(&default.tagline).to_owned(),
            about: document.get_str("about").unwrap_or_default().to_owned(),
            theme: document.get_str("theme").unwrap_or(&default.theme).to_owned(),
            font_family: document.get_str("fontFamily").unwrap_or(&default.font_family).to_owned(),
            layout_style: document.get_str("layoutStyle").unwrap_or(&default.layout_style).to_owned(),
            visual_effect: document.get_str("visualEffect").unwrap_or(&default.visual_effect).to_owned(),
            brand_color: document.get_str("brandColor").unwrap_or(&default.brand_color).to_owned(),
            accent_color: document.get_str("accentColor").unwrap_or(&default.accent_color).to_owned(),
            logo_url: document.get_str("logoUrl").ok().map(str::to_owned),
            hero_image_url: document.get_str("heroImageUrl").ok().map(str::to_owned),
            contact_email: document.get_str("contactEmail").ok().map(str::to_owned),
            is_published: document.get_bool("isPublished").unwrap_or(false),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicCourse {
    id: String,
    title: String,
    slug: String,
    description: Option<String>,
    price_in_paise: i64,
    currency: String,
    thumbnail_url: Option<String>,
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value[1..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_image_url(value: &Option<String>) -> bool {
    value.as_ref().is_none_or(|url| {
        url.trim().is_empty()
            || (url.starts_with("https://") && url.len() <= 2048)
    })
}

fn validate(input: &UpdateSiteRequest) -> AppResult<()> {
    let display = input.display_name.trim();
    let headline = input.headline.trim();
    let tagline = input.tagline.trim();
    let about = input.about.trim();
    let email_ok = input.contact_email.as_ref().is_none_or(|email| {
        let value = email.trim();
        value.is_empty()
            || (value.len() <= 254 && value.contains('@') && !value.contains(' '))
    });

    if !(2..=100).contains(&display.chars().count())
        || !(2..=160).contains(&headline.chars().count())
        || tagline.chars().count() > 240
        || about.chars().count() > 10_000
        || !matches!(input.theme.as_str(), "ocean" | "forest" | "sunset" | "midnight" | "minimal" | "aurora" | "lavender" | "monochrome" | "coral")
        || !matches!(input.font_family.as_str(), "sans" | "rounded" | "serif" | "editorial" | "mono")
        || !matches!(input.layout_style.as_str(), "split" | "centered" | "editorial" | "immersive")
        || !matches!(input.visual_effect.as_str(), "none" | "glass" | "gradient" | "glow")
        || !is_hex_color(&input.brand_color)
        || !is_hex_color(&input.accent_color)
        || !valid_image_url(&input.logo_url)
        || !valid_image_url(&input.hero_image_url)
        || !email_ok
    {
        return Err(AppError::BadRequest("site fields do not meet validation rules"));
    }
    Ok(())
}

pub async fn get_company_site(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<Value>> {
    if !matches!(auth.role.as_str(), "owner" | "admin") {
        return Err(AppError::Forbidden);
    }
    let company = state.db.collection::<Document>("companies")
        .find_one(doc! { "_id": auth.company_id.clone(), "status": "active" })
        .await?.ok_or(AppError::NotFound)?;
    let company_name = company.get_str("name").unwrap_or("Company");
    let site = state.db.collection::<Document>("company_sites")
        .find_one(doc! { "companyId": auth.company_id.clone() })
        .await?;
    let view = site.as_ref().map(|doc| SiteView::from_document(doc, company_name))
        .unwrap_or_else(|| SiteView::default_for(company_name));
    Ok(Json(json!({
        "site": view,
        "company": { "name": company_name, "slug": company.get_str("slug").unwrap_or_default() },
        "publicPath": format!("/p/{}", company.get_str("slug").unwrap_or_default())
    })))
}

pub async fn update_company_site(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(input): Json<UpdateSiteRequest>,
) -> AppResult<Json<Value>> {
    if !matches!(auth.role.as_str(), "owner" | "admin") {
        return Err(AppError::Forbidden);
    }
    validate(&input)?;
    let company = state.db.collection::<Document>("companies")
        .find_one(doc! { "_id": auth.company_id.clone(), "status": "active" })
        .await?.ok_or(AppError::NotFound)?;
    let company_name = company.get_str("name").unwrap_or("Company");
    let site_collection = state.db.collection::<Document>("company_sites");
    let filter = doc! { "companyId": auth.company_id.clone() };
    let site_doc = doc! {
        "companyId": auth.company_id.clone(),
        "displayName": input.display_name.trim(),
        "headline": input.headline.trim(),
        "tagline": input.tagline.trim(),
        "about": input.about.trim(),
        "theme": &input.theme,
        "fontFamily": &input.font_family,
        "layoutStyle": &input.layout_style,
        "visualEffect": &input.visual_effect,
        "brandColor": input.brand_color.to_ascii_lowercase(),
        "accentColor": input.accent_color.to_ascii_lowercase(),
        "logoUrl": input.logo_url.as_deref().map(str::trim).filter(|v| !v.is_empty()),
        "heroImageUrl": input.hero_image_url.as_deref().map(str::trim).filter(|v| !v.is_empty()),
        "contactEmail": input.contact_email.as_deref().map(str::trim).filter(|v| !v.is_empty()),
        "isPublished": input.is_published,
        "updatedAt": DateTime::now(),
    };
    if site_collection.find_one(filter.clone()).await?.is_some() {
        site_collection.update_one(filter.clone(), doc! { "$set": site_doc }).await?;
    } else {
        let mut insert = site_doc;
        insert.insert("_id", ObjectId::new());
        insert.insert("createdAt", DateTime::now());
        site_collection.insert_one(insert).await.map_err(AppError::from_mongo)?;
    }
    let saved = site_collection.find_one(filter).await?.ok_or(AppError::NotFound)?;
    let view = SiteView::from_document(&saved, company_name);
    Ok(Json(json!({
        "site": view,
        "company": { "name": company_name, "slug": company.get_str("slug").unwrap_or_default() },
        "publicPath": format!("/p/{}", company.get_str("slug").unwrap_or_default())
    })))
}

pub async fn get_public_company_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> AppResult<Json<Value>> {
    let slug = slug.trim().to_ascii_lowercase();
    if slug.is_empty() || slug.len() > 80 || !slug.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-') {
        return Err(AppError::NotFound);
    }
    let company = state.db.collection::<Document>("companies")
        .find_one(doc! { "slug": &slug, "status": "active" })
        .await?.ok_or(AppError::NotFound)?;
    let company_id = company.get_object_id("_id").map_err(|_| AppError::Internal)?.to_owned();
    let company_name = company.get_str("name").unwrap_or("Company").to_owned();
    let site_document = state.db.collection::<Document>("company_sites")
        .find_one(doc! { "companyId": company_id.clone(), "isPublished": true })
        .await?.ok_or(AppError::NotFound)?;
    let site = SiteView::from_document(&site_document, &company_name);

    let cursor = state.db.collection::<Document>("courses")
        .find(doc! {
            "companyId": company_id,
            "status": "published",
            "deletedAt": { "$exists": false },
        })
        .sort(doc! { "createdAt": -1 })
        .limit(100)
        .await?;
    let documents: Vec<Document> = cursor.try_collect().await.map_err(AppError::from_mongo)?;
    let courses: Vec<PublicCourse> = documents.into_iter().filter_map(|document| {
        let id = document.get_object_id("_id").ok()?.to_hex();
        Some(PublicCourse {
            id,
            title: document.get_str("title").ok()?.to_owned(),
            slug: document.get_str("slug").ok()?.to_owned(),
            description: document.get_str("description").ok().map(str::to_owned),
            price_in_paise: document.get_i64("priceInPaise").unwrap_or(0),
            currency: document.get_str("currency").unwrap_or("INR").to_owned(),
            thumbnail_url: document.get_str("thumbnailUrl").ok().map(str::to_owned),
        })
    }).collect();

    Ok(Json(json!({
        "company": { "name": company_name, "slug": slug },
        "site": site,
        "publicPath": format!("/p/{slug}"),
        "courses": courses,
    })))
}
