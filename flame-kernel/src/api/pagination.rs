use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

// ── Pagination ────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema, IntoParams)]
pub struct PaginationParams {
    /// 页码（从 1 开始）
    #[param(minimum = 1, default = 1)]
    pub page: Option<i64>,
    /// 每页条数（1~200）
    #[param(minimum = 1, maximum = 200, default = 20)]
    pub page_size: Option<i64>,
}

impl PaginationParams {
    pub fn page(&self) -> i64 {
        self.page.filter(|p| *p > 0).unwrap_or(1)
    }
    pub fn page_size(&self) -> i64 {
        self.page_size.filter(|s| *s > 0 && *s <= 200).unwrap_or(20)
    }
    pub fn offset(&self) -> i64 {
        (self.page() - 1) * self.page_size()
    }
}

#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub data: Vec<T>,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
    pub total_pages: i64,
}

// utoipa 泛型 schema：实现 ComposeSchema（自动提供 PartialSchema），并补充 ToSchema::schemas
impl<T: Serialize + utoipa::ToSchema> utoipa::__dev::ComposeSchema for PaginatedResponse<T> {
    fn compose(
        new_generics: Vec<utoipa::openapi::RefOr<utoipa::openapi::Schema>>,
    ) -> utoipa::openapi::RefOr<utoipa::openapi::Schema> {
        use utoipa::openapi::schema::Type;
        use utoipa::openapi::ObjectBuilder;
        let data_schema = new_generics
            .into_iter()
            .next()
            .unwrap_or_else(|| T::schema());
        ObjectBuilder::new()
            .schema_type(Type::Object)
            .property(
                "data",
                utoipa::openapi::ArrayBuilder::new()
                    .items(data_schema)
                    .build(),
            )
            .required("data")
            .property(
                "page",
                ObjectBuilder::new().schema_type(Type::Integer).build(),
            )
            .required("page")
            .property(
                "page_size",
                ObjectBuilder::new().schema_type(Type::Integer).build(),
            )
            .required("page_size")
            .property(
                "total",
                ObjectBuilder::new().schema_type(Type::Integer).build(),
            )
            .required("total")
            .property(
                "total_pages",
                ObjectBuilder::new().schema_type(Type::Integer).build(),
            )
            .required("total_pages")
            .into()
    }
}

impl<T: Serialize + utoipa::ToSchema> utoipa::ToSchema for PaginatedResponse<T> {
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(format!("PaginatedResponse<{}>", T::name()))
    }

    fn schemas(schemas: &mut Vec<(String, utoipa::openapi::RefOr<utoipa::openapi::Schema>)>) {
        <T as utoipa::ToSchema>::schemas(schemas);
        let item = T::schema();
        let object = utoipa::openapi::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::Object)
            .property(
                "data",
                utoipa::openapi::ArrayBuilder::new().items(item).build(),
            )
            .required("data")
            .property(
                "page",
                utoipa::openapi::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::Type::Integer)
                    .build(),
            )
            .required("page")
            .property(
                "page_size",
                utoipa::openapi::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::Type::Integer)
                    .build(),
            )
            .required("page_size")
            .property(
                "total",
                utoipa::openapi::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::Type::Integer)
                    .build(),
            )
            .required("total")
            .property(
                "total_pages",
                utoipa::openapi::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::Type::Integer)
                    .build(),
            )
            .required("total_pages")
            .into();
        schemas.push((Self::name().into_owned(), object));
    }
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: i64, params: &PaginationParams) -> Self {
        let page = params.page();
        let page_size = params.page_size();
        let total_pages = if page_size > 0 {
            (total + page_size - 1) / page_size
        } else {
            1
        };
        Self {
            data: items,
            page,
            page_size,
            total,
            total_pages,
        }
    }
}
