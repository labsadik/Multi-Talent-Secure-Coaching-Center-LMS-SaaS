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
        &assets,
        IndexModel::builder()
            .keys(doc! { "companyId": 1, "courseId": 1, "topicId": 1, "status": 1 })
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
