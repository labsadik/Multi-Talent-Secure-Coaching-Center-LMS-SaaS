#!/usr/bin/env bash
set -Eeuo pipefail

# Run only against a disposable development/test database. This script creates
# two companies, a learner, and a course; it intentionally does not clean them up.
BASE_URL="${BASE_URL:-http://127.0.0.1:8080}"
BASE_URL="${BASE_URL%/}"

for dependency in curl jq; do
  if ! command -v "$dependency" >/dev/null 2>&1; then
    printf 'Missing required command: %s\n' "$dependency" >&2
    exit 2
  fi
done

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

fail() {
  # Never print request arguments or response bodies: they may contain passwords,
  # one-time access tokens, provider data, or other sensitive values.
  printf 'FAIL: %s\n' "$1" >&2
  exit 1
}

http() {
  local expected_status="$1"
  shift
  local target="${1:-<unknown>}"
  local actual_status
  if ! actual_status="$(curl --connect-timeout 3 --max-time 20 -sS \
    -D "$TMP_DIR/headers" -o "$TMP_DIR/body" -w '%{http_code}' "$@")"; then
    fail "request failed for $target"
  fi
  if [[ "$actual_status" != "$expected_status" ]]; then
    fail "expected HTTP $expected_status, got $actual_status for $target"
  fi
}

assert_json() {
  local file="$1"
  local expression="$2"
  local message="$3"
  jq -e "$expression" "$file" >/dev/null || fail "$message"
}

http 200 "$BASE_URL/health/live"
cp "$TMP_DIR/body" "$TMP_DIR/live.json"
assert_json "$TMP_DIR/live.json" '.status == "live"' "liveness response is invalid"
grep -Eiq '^x-content-type-options:[[:space:]]*nosniff' "$TMP_DIR/headers" || fail "missing nosniff header"
grep -Eiq '^x-frame-options:[[:space:]]*DENY' "$TMP_DIR/headers" || fail "missing frame-deny header"
grep -Eiq '^cache-control:[[:space:]]*no-store' "$TMP_DIR/headers" || fail "missing no-store header"

http 200 "$BASE_URL/health/ready"
cp "$TMP_DIR/body" "$TMP_DIR/ready.json"
assert_json "$TMP_DIR/ready.json" '.status == "ready" and .database == "ok"' "readiness/database check failed"

http 200 "$BASE_URL/"
cp "$TMP_DIR/body" "$TMP_DIR/root.json"
assert_json "$TMP_DIR/root.json" '.service == "newage-api" and .status == "running"' "root service response is invalid"

http 401 "$BASE_URL/api/v1/auth/me"
cp "$TMP_DIR/body" "$TMP_DIR/unauthorized.json"
assert_json "$TMP_DIR/unauthorized.json" '.error.code == "unauthorized"' "unauthenticated request was not rejected consistently"

http 404 "$BASE_URL/api/v1/__smoke_test_missing_route__"
cp "$TMP_DIR/body" "$TMP_DIR/not-found.json"
assert_json "$TMP_DIR/not-found.json" '.error.code == "not_found"' "unknown route did not return the expected JSON error"

suffix="$(date +%s)-$RANDOM"
password='Smoke-Test-Password-2026!'
slug_a="smoke-company-a-$suffix"
slug_b="smoke-company-b-$suffix"
email_a="owner-a-$suffix@example.test"
email_b="owner-b-$suffix@example.test"
learner_email="learner-a-$suffix@example.test"
instructor_email="instructor-a-$suffix@example.test"
managed_learner_email="managed-learner-a-$suffix@example.test"
admin_email="admin-a-$suffix@example.test"
admin_created_by_admin_email="admin-created-by-admin-$suffix@example.test"

payload="$(jq -n --arg name 'Smoke Test Company A' --arg slug "$slug_a" --arg email "$email_a" --arg password "$password" '{companyName:$name,companySlug:$slug,name:"Smoke Test Owner A",email:$email,password:$password}')"
http 201 "$BASE_URL/api/v1/auth/register" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/register-a.json"
assert_json "$TMP_DIR/register-a.json" '.user.role == "owner" and .company.slug != null and .accessToken != null' "company A registration response is invalid"

payload="$(jq -n --arg name 'Smoke Test Company B' --arg slug "$slug_b" --arg email "$email_b" --arg password "$password" '{companyName:$name,companySlug:$slug,name:"Smoke Test Owner B",email:$email,password:$password}')"
http 201 "$BASE_URL/api/v1/auth/register" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/register-b.json"
assert_json "$TMP_DIR/register-b.json" '.user.role == "owner" and .company.slug != null and .accessToken != null' "company B registration response is invalid"

payload="$(jq -n --arg slug "$slug_a" --arg email "$email_a" --arg password "$password" '{companySlug:$slug,email:$email,password:$password}')"
http 200 "$BASE_URL/api/v1/auth/login" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/login-a.json"
token_a="$(jq -er '.accessToken | select(type == "string" and length > 0)' "$TMP_DIR/login-a.json")"
refresh_a="$(jq -er '.refreshToken | select(type == "string" and length >= 40)' "$TMP_DIR/login-a.json")"
assert_json "$TMP_DIR/login-a.json" '.user.role == "owner" and .refreshExpiresIn >= 86400' "company A login response is invalid"

# Refresh renews the access token while retaining the same opaque session credential.
payload="$(jq -n --arg refreshToken "$refresh_a" '{refreshToken:$refreshToken}')"
http 200 "$BASE_URL/api/v1/auth/refresh" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/refresh-a.json"
token_a="$(jq -er '.accessToken | select(type == "string" and length > 0)' "$TMP_DIR/refresh-a.json")"
jq -e --arg token "$refresh_a" '.refreshToken == $token' "$TMP_DIR/refresh-a.json" >/dev/null || fail "refresh token changed unexpectedly"

payload="$(jq -n --arg slug "$slug_b" --arg email "$email_b" --arg password "$password" '{companySlug:$slug,email:$email,password:$password}')"
http 200 "$BASE_URL/api/v1/auth/login" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/login-b.json"
token_b="$(jq -er '.accessToken | select(type == "string" and length > 0)' "$TMP_DIR/login-b.json")"
assert_json "$TMP_DIR/login-b.json" '.user.role == "owner"' "company B login response is invalid"

http 200 "$BASE_URL/api/v1/auth/me" -H "Authorization: Bearer $token_a"
cp "$TMP_DIR/body" "$TMP_DIR/me-a.json"
jq -e --arg slug "$slug_a" '.company.slug == $slug and .user.role == "owner"' "$TMP_DIR/me-a.json" >/dev/null || fail "authenticated identity does not match company A"

# Instructors can manage learners, but cannot create/manage staff accounts.
payload="$(jq -n --arg email "$instructor_email" --arg password "$password" '{name:"Smoke Test Instructor",email:$email,password:$password,role:"instructor"}')"
http 201 "$BASE_URL/api/v1/company/users" -X POST -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/instructor-created.json"
instructor_id="$(jq -er '.id | select(type == "string" and length > 0)' "$TMP_DIR/instructor-created.json")"
assert_json "$TMP_DIR/instructor-created.json" '.role == "instructor"' "owner could not create an instructor"

payload="$(jq -n --arg slug "$slug_a" --arg email "$instructor_email" --arg password "$password" '{companySlug:$slug,email:$email,password:$password}')"
http 200 "$BASE_URL/api/v1/auth/login" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/instructor-login.json"
token_instructor="$(jq -er '.accessToken | select(type == "string" and length > 0)' "$TMP_DIR/instructor-login.json")"
assert_json "$TMP_DIR/instructor-login.json" '.user.role == "instructor"' "instructor login failed"

payload="$(jq -n --arg email "$managed_learner_email" --arg password "$password" '{name:"Managed Smoke Learner",email:$email,password:$password,role:"learner"}')"
http 201 "$BASE_URL/api/v1/company/users" -X POST -H "Authorization: Bearer $token_instructor" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/managed-learner-created.json"
managed_learner_id="$(jq -er '.id | select(type == "string" and length > 0)' "$TMP_DIR/managed-learner-created.json")"
assert_json "$TMP_DIR/managed-learner-created.json" '.role == "learner"' "instructor could not create a learner"

http 200 "$BASE_URL/api/v1/company/users" -H "Authorization: Bearer $token_instructor"
cp "$TMP_DIR/body" "$TMP_DIR/instructor-users.json"
assert_json "$TMP_DIR/instructor-users.json" 'length == 1 and all(.[]; .role == "learner")' "instructor listing exposed non-learner accounts"

payload="$(jq -n --arg email "staff-blocked-$suffix@example.test" --arg password "$password" '{name:"Blocked Staff",email:$email,password:$password,role:"instructor"}')"
http 403 "$BASE_URL/api/v1/company/users" -X POST -H "Authorization: Bearer $token_instructor" -H 'Content-Type: application/json' --data "$payload"
payload="$(jq -n '{role:"instructor"}')"
http 403 "$BASE_URL/api/v1/company/users/'"$managed_learner_id"'" -X PATCH -H "Authorization: Bearer $token_instructor" -H 'Content-Type: application/json' --data "$payload"
payload="$(jq -n '{status:"disabled"}')"
http 200 "$BASE_URL/api/v1/company/users/$managed_learner_id" -X PATCH -H "Authorization: Bearer $token_instructor" -H 'Content-Type: application/json' --data "$payload"
assert_json "$TMP_DIR/body" '.role == "learner" and .status == "disabled"' "instructor could not disable a learner"

# Owners and admins have full workspace-user management, including assigning admin.
payload="$(jq -n --arg email "$admin_email" --arg password "$password" '{name:"Smoke Test Admin",email:$email,password:$password,role:"admin"}')"
http 201 "$BASE_URL/api/v1/company/users" -X POST -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/admin-created.json"
assert_json "$TMP_DIR/admin-created.json" '.role == "admin"' "owner could not create an admin"

payload="$(jq -n --arg slug "$slug_a" --arg email "$admin_email" --arg password "$password" '{companySlug:$slug,email:$email,password:$password}')"
http 200 "$BASE_URL/api/v1/auth/login" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/admin-login.json"
token_admin="$(jq -er '.accessToken | select(type == "string" and length > 0)' "$TMP_DIR/admin-login.json")"
assert_json "$TMP_DIR/admin-login.json" '.user.role == "admin"' "admin login failed"

payload="$(jq -n --arg email "$admin_created_by_admin_email" --arg password "$password" '{name:"Admin Created By Admin",email:$email,password:$password,role:"admin"}')"
http 201 "$BASE_URL/api/v1/company/users" -X POST -H "Authorization: Bearer $token_admin" -H 'Content-Type: application/json' --data "$payload"
assert_json "$TMP_DIR/body" '.role == "admin"' "admin does not have full user-management permissions"
http 403 "$BASE_URL/api/v1/company/users/$admin_id" -X PATCH -H "Authorization: Bearer $token_instructor" -H 'Content-Type: application/json' --data "$(jq -n '{status:"disabled"}')"

payload="$(jq -n --arg slug "smoke-course-$suffix" '{title:"Tenant Isolation Smoke Course",slug:$slug,description:"Automated API smoke test",priceInPaise:0}')"
http 201 "$BASE_URL/api/v1/courses" -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/course.json"
course_id="$(jq -er '.id | select(type == "string" and length > 0)' "$TMP_DIR/course.json")"
assert_json "$TMP_DIR/course.json" '.status == "draft" and .title == "Tenant Isolation Smoke Course"' "course creation response is invalid"

# A user in company B must not publish company A's course, even when its ID is known.
http 404 "$BASE_URL/api/v1/courses/$course_id/publish" -X POST -H "Authorization: Bearer $token_b"
cp "$TMP_DIR/body" "$TMP_DIR/cross-tenant.json"
assert_json "$TMP_DIR/cross-tenant.json" '.error.code == "not_found"' "cross-tenant course access was not blocked"

http 200 "$BASE_URL/api/v1/courses/$course_id/publish" -X POST -H "Authorization: Bearer $token_a"
cp "$TMP_DIR/body" "$TMP_DIR/published.json"
assert_json "$TMP_DIR/published.json" '.status == "published" and .id != null' "course publish failed for its owning tenant"

http 200 "$BASE_URL/api/v1/courses" -H "Authorization: Bearer $token_b"
cp "$TMP_DIR/body" "$TMP_DIR/courses-b.json"
if jq -e --arg id "$course_id" 'any(.[]; .id == $id)' "$TMP_DIR/courses-b.json" >/dev/null; then
  fail "company B can see company A's course"
fi

http 200 "$BASE_URL/api/v1/courses" -H "Authorization: Bearer $token_a"
cp "$TMP_DIR/body" "$TMP_DIR/courses-a.json"
jq -e --arg id "$course_id" 'any(.[]; .id == $id and .status == "published")' "$TMP_DIR/courses-a.json" >/dev/null || fail "company A cannot see its own published course"

# Metadata and nested content must support real PATCH/DELETE operations for the owning tenant.
payload="$(jq -n --arg slug "smoke-course-edited-$suffix" '{title:"Edited Smoke Course",slug:$slug,description:"Updated from smoke test",priceInPaise:1299}')"
http 200 "$BASE_URL/api/v1/courses/$course_id" -X PATCH -H "Authorization: Bearer $token_instructor" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/course-edited.json"
assert_json "$TMP_DIR/course-edited.json" '.title == "Edited Smoke Course" and .slug == ("smoke-course-edited-" + "'"$suffix"'") and .priceInPaise == 1299' "course edit did not persist"
http 404 "$BASE_URL/api/v1/courses/$course_id" -X PATCH -H "Authorization: Bearer $token_b" -H 'Content-Type: application/json' --data "$payload"

payload="$(jq -n --arg slug "smoke-subject-$suffix" '{title:"Smoke Subject",slug:$slug,description:"Initial subject",sortOrder:0}')"
http 201 "$BASE_URL/api/v1/courses/$course_id/subjects" -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/subject.json"
subject_id="$(jq -er '.id | select(type == "string" and length > 0)' "$TMP_DIR/subject.json")"
payload="$(jq -n --arg slug "smoke-subject-edited-$suffix" '{title:"Edited Smoke Subject",slug:$slug,description:"Edited subject",sortOrder:1}')"
http 200 "$BASE_URL/api/v1/subjects/$subject_id" -X PATCH -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/subject-edited.json"
assert_json "$TMP_DIR/subject-edited.json" '.title == "Edited Smoke Subject" and .sortOrder == 1' "subject edit did not persist"

payload="$(jq -n --arg slug "smoke-topic-$suffix" '{title:"Smoke Topic",slug:$slug,description:"Initial topic",sortOrder:0}')"
http 201 "$BASE_URL/api/v1/subjects/$subject_id/topics" -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/topic.json"
topic_id="$(jq -er '.id | select(type == "string" and length > 0)' "$TMP_DIR/topic.json")"
payload="$(jq -n --arg slug "smoke-topic-edited-$suffix" '{title:"Edited Smoke Topic",slug:$slug,description:"Edited topic",sortOrder:2}')"
http 200 "$BASE_URL/api/v1/topics/$topic_id" -X PATCH -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/topic-edited.json"
assert_json "$TMP_DIR/topic-edited.json" '.title == "Edited Smoke Topic" and .sortOrder == 2' "topic edit did not persist"

# Build and verify the planned Subject -> Part -> Chapter -> Chapter part -> lesson hierarchy.
payload="$(jq -n --arg slug "smoke-chapter-$suffix" '{title:"Smoke Chapter",slug:$slug,description:"Chapter test",sortOrder:0}')"
http 201 "$BASE_URL/api/v1/topics/$topic_id/chapters" -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/chapter.json"
chapter_id="$(jq -er '.id | select(type == "string" and length > 0)' "$TMP_DIR/chapter.json")"
payload="$(jq -n --arg slug "smoke-chapter-edited-$suffix" '{title:"Edited Smoke Chapter",slug:$slug,description:"Updated chapter",sortOrder:1}')"
http 200 "$BASE_URL/api/v1/chapters/$chapter_id" -X PATCH -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/chapter-edited.json"
assert_json "$TMP_DIR/chapter-edited.json" '.title == "Edited Smoke Chapter" and .sortOrder == 1' "chapter edit did not persist"

payload="$(jq -n --arg slug "smoke-chapter-part-$suffix" '{title:"Smoke Chapter Part",slug:$slug,description:"Nested chapter section",sortOrder:0}')"
http 201 "$BASE_URL/api/v1/chapters/$chapter_id/parts" -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/chapter-part.json"
chapter_part_id="$(jq -er '.id | select(type == "string" and length > 0)' "$TMP_DIR/chapter-part.json")"
payload="$(jq -n --arg slug "smoke-chapter-part-edited-$suffix" '{title:"Edited Chapter Part",slug:$slug,description:"Updated nested section",sortOrder:2}')"
http 200 "$BASE_URL/api/v1/chapter-parts/$chapter_part_id" -X PATCH -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/chapter-part-edited.json"
assert_json "$TMP_DIR/chapter-part-edited.json" '.title == "Edited Chapter Part" and .sortOrder == 2' "chapter part edit did not persist"

payload="$(jq -n --arg slug "smoke-nested-lesson-$suffix" '{title:"Nested Smoke Lesson",slug:$slug,lessonType:"text",content:"Lesson inside a chapter part",durationSeconds:45,isPreview:true,sortOrder:0}')"
http 201 "$BASE_URL/api/v1/chapter-parts/$chapter_part_id/lessons" -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/nested-lesson.json"
nested_lesson_id="$(jq -er '.id | select(type == "string" and length > 0)' "$TMP_DIR/nested-lesson.json")"
jq -e --arg chapter "$chapter_id" --arg part "$chapter_part_id" '.chapterId == $chapter and .chapterPartId == $part and .lessonType == "text"' "$TMP_DIR/nested-lesson.json" >/dev/null || fail "nested lesson did not retain its chapter hierarchy"
http 200 "$BASE_URL/api/v1/chapter-parts/$chapter_part_id/lessons" -H "Authorization: Bearer $token_a"
jq -e --arg id "$nested_lesson_id" 'length == 1 and .[0].id == $id' "$TMP_DIR/body" >/dev/null || fail "nested chapter-part lessons were not listed"
http 204 "$BASE_URL/api/v1/chapter-parts/$chapter_part_id" -X DELETE -H "Authorization: Bearer $token_a"
http 200 "$BASE_URL/api/v1/topics/$topic_id/lessons" -H "Authorization: Bearer $token_a"
assert_json "$TMP_DIR/body" 'length == 0' "archived chapter-part lesson is still listed"

payload="$(jq -n --arg slug "smoke-lesson-$suffix" '{title:"Smoke Lesson",slug:$slug,lessonType:"text",content:"Original lesson body",durationSeconds:60,isPreview:true,sortOrder:0}')"
http 201 "$BASE_URL/api/v1/topics/$topic_id/lessons" -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/lesson.json"
lesson_id="$(jq -er '.id | select(type == "string" and length > 0)' "$TMP_DIR/lesson.json")"
payload="$(jq -n --arg slug "smoke-lesson-edited-$suffix" '{title:"Edited Smoke Lesson",slug:$slug,lessonType:"text",content:"Updated lesson body",durationSeconds:90,isPreview:false,sortOrder:3}')"
http 200 "$BASE_URL/api/v1/lessons/$lesson_id" -X PATCH -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/lesson-edited.json"
assert_json "$TMP_DIR/lesson-edited.json" '.title == "Edited Smoke Lesson" and .content == "Updated lesson body" and .durationSeconds == 90 and .isPreview == false' "lesson edit did not persist"
http 204 "$BASE_URL/api/v1/lessons/$lesson_id" -X DELETE -H "Authorization: Bearer $token_a"
http 200 "$BASE_URL/api/v1/topics/$topic_id/lessons" -H "Authorization: Bearer $token_a"
assert_json "$TMP_DIR/body" 'length == 0' "archived lesson is still listed"

# A company owner can brand/publish a tenant-scoped storefront with its published course catalogue.
payload="$(jq -n --arg display "Smoke Test Public Academy" --arg headline "Learn with Smoke Academy" --arg email "contact-$suffix@example.test" '{displayName:$display,headline:$headline,tagline:"Learn at your pace",about:"Public company page smoke test",theme:"ocean",brandColor:"#2563eb",accentColor:"#14b8a6",logoUrl:null,heroImageUrl:null,contactEmail:$email,isPublished:true}')"
http 403 "$BASE_URL/api/v1/company/site" -X PUT -H "Authorization: Bearer $token_instructor" -H 'Content-Type: application/json' --data "$payload"
http 200 "$BASE_URL/api/v1/company/site" -X PUT -H "Authorization: Bearer $token_admin" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/site-saved.json"
jq -e --arg path "/p/$slug_a" '.site.isPublished == true and .site.displayName == "Smoke Test Public Academy" and .publicPath == $path' "$TMP_DIR/site-saved.json" >/dev/null || fail "company public site configuration did not persist"
http 200 "$BASE_URL/api/v1/public/companies/$slug_a"
cp "$TMP_DIR/body" "$TMP_DIR/public-company.json"
jq -e --arg slug "$slug_a" --arg course "$course_id" '.site.isPublished == true and .company.slug == $slug and any(.courses[]; .id == $course)' "$TMP_DIR/public-company.json" >/dev/null || fail "public company page did not return the published course"
http 404 "$BASE_URL/api/v1/public/companies/unknown-company-$suffix"

# Subject archival cascades to active nested topics, lessons, and asset metadata.
http 204 "$BASE_URL/api/v1/subjects/$subject_id" -X DELETE -H "Authorization: Bearer $token_a"
http 200 "$BASE_URL/api/v1/courses/$course_id/subjects" -H "Authorization: Bearer $token_a"
assert_json "$TMP_DIR/body" 'length == 0' "archived subject is still listed"
http 200 "$BASE_URL/api/v1/courses/$course_id" -X DELETE -H "Authorization: Bearer $token_a"
cp "$TMP_DIR/body" "$TMP_DIR/course-archived.json"
assert_json "$TMP_DIR/course-archived.json" '.status == "archived"' "course archival failed"
http 200 "$BASE_URL/api/v1/courses" -H "Authorization: Bearer $token_a"
if jq -e --arg id "$course_id" 'any(.[]; .id == $id)' "$TMP_DIR/body" >/dev/null; then
  fail "archived course still appears in the course listing"
fi

payload="$(jq -n --arg slug "$slug_a" --arg email "$learner_email" --arg password "$password" '{companySlug:$slug,name:"Smoke Test Learner",email:$email,password:$password}')"
http 201 "$BASE_URL/api/v1/auth/register-learner" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/learner.json"
learner_token="$(jq -er '.accessToken | select(type == "string" and length > 0)' "$TMP_DIR/learner.json")"
assert_json "$TMP_DIR/learner.json" '.user.role == "learner"' "learner registration response is invalid"

payload="$(jq -n --arg slug "learner-must-not-create-$suffix" '{title:"Learner Must Not Create",slug:$slug,priceInPaise:0}')"
http 403 "$BASE_URL/api/v1/courses" -H "Authorization: Bearer $learner_token" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/learner-forbidden.json"
assert_json "$TMP_DIR/learner-forbidden.json" '.error.code == "forbidden"' "learner was allowed to create a course"
http 200 "$BASE_URL/api/v1/courses" -H "Authorization: Bearer $learner_token"
http 200 "$BASE_URL/api/v1/me/courses" -H "Authorization: Bearer $learner_token"
http 403 "$BASE_URL/api/v1/company/users" -H "Authorization: Bearer $learner_token"

payload="$(jq -n --arg refreshToken "$refresh_a" '{refreshToken:$refreshToken}')"
http 204 "$BASE_URL/api/v1/auth/logout" -H 'Content-Type: application/json' --data "$payload"
http 401 "$BASE_URL/api/v1/auth/refresh" -H 'Content-Type: application/json' --data "$payload"

printf 'PASS: health, readiness, security headers, auth guard, registration/login, refresh/logout, learner RBAC, course lifecycle, and tenant isolation.\n'
