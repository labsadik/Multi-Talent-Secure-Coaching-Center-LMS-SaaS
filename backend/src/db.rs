use mongodb::{
    bson::doc,
    options::IndexOptions,
    results::CreateIndexResult,
    Collection, Database, IndexModel,
};

pub async fn ensure_indexes(db: &Database) -> mongodb::error::Result<()> {
    let companies: Collection<mongodb::bson::Document> = db.collection("companies");
    let users: Collection<mongodb::bson::Document> = db.collection("users");
    let courses: Collection<mongodb::bson::Document> = db.collection("courses");
    let subjects: Collection<mongodb::bson::Document> = db.collection("subjects");
    let topics: Collection<mongodb::bson::Document> = db.collection("topics");
    let lessons: Collection<mongodb::bson::Document> = db.collection("lessons");
    let assets: Collection<mongodb::bson::Document> = db.collection("assets");
    let orders: Collection<mongodb::bson::Document> = db.collection("orders");
    let payments: Collection<mongodb::bson::Document> = db.collection("payments");
    let enrollments: Collection<mongodb::bson::Document> = db.collection("enrollments");
    let lesson_progress: Collection<mongodb::bson::Document> = db.collection("lesson_progress");
    let subscription_orders: Collection<mongodb::bson::Document> = db.collection("subscription_orders");
    let subscription_payments: Collection<mongodb::bson::Document> = db.collection("subscription_payments");
    let refresh_sessions: Collection<mongodb::bson::Document> = db.collection("refresh_sessions");
    let company_sites: Collection<mongodb::bson::Document> = db.collection("company_sites");

    create_index(
        &company_sites,
        IndexModel::builder()
            .keys(doc! { "companyId": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;


    create_index(
        &refresh_sessions,
        IndexModel::builder()
            .keys(doc! { "tokenHash": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &refresh_sessions,
        IndexModel::builder()
            .keys(doc! { "expiresAt": 1 })
            .options(IndexOptions::builder().expire_after(std::time::Duration::ZERO).build())
            .build(),
    )
    .await?;

    create_index(
        &companies,
        IndexModel::builder()
            .keys(doc! { "slug": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &users,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "email": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &courses,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "slug": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &subjects,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "courseId": 1, "slug": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &topics,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "subjectId": 1, "slug": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &lessons,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "topicId": 1, "slug": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &assets,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "courseId": 1, "topicId": 1, "status": 1 })
            .build(),
    )
    .await?;

    create_index(
        &orders,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "razorpayOrderId": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &payments,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "razorpayPaymentId": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &enrollments,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "courseId": 1, "learnerId": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &lesson_progress,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "learnerId": 1, "lessonId": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &lesson_progress,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "courseId": 1, "learnerId": 1, "status": 1 })
            .build(),
    )
    .await?;

    create_index(
        &subscription_orders,
        IndexModel::builder()
            .keys(doc! { "razorpayOrderId": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    create_index(
        &subscription_orders,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "status": 1, "createdAt": -1 })
            .build(),
    )
    .await?;

    create_index(
        &subscription_payments,
        IndexModel::builder()
            .keys(doc! { "razorpayPaymentId": 1 })
            .options(IndexOptions::builder().unique(true).build())
            .build(),
    )
    .await?;

    Ok(())
}

async fn create_index<T>(
    collection: &Collection<T>,
    model: IndexModel,
) -> mongodb::error::Result<CreateIndexResult>
where
    T: Send + Sync,
{
    collection.create_index(model).await
}
