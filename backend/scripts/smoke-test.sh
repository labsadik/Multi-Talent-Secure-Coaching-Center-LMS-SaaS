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
  printf 'FAIL: %s\n' "$*" >&2
  if [[ -f "$TMP_DIR/body" ]]; then
    cat "$TMP_DIR/body" >&2
    printf '\n' >&2
  fi
  exit 1
}

http() {
  local expected_status="$1"
  shift
  local actual_status
  if ! actual_status="$(curl --connect-timeout 3 --max-time 20 -sS \
    -D "$TMP_DIR/headers" -o "$TMP_DIR/body" -w '%{http_code}' "$@")"; then
    fail "request failed: $*"
  fi
  if [[ "$actual_status" != "$expected_status" ]]; then
    fail "expected HTTP $expected_status, got $actual_status for $*"
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
assert_json "$TMP_DIR/login-a.json" '.user.role == "owner"' "company A login response is invalid"

payload="$(jq -n --arg slug "$slug_b" --arg email "$email_b" --arg password "$password" '{companySlug:$slug,email:$email,password:$password}')"
http 200 "$BASE_URL/api/v1/auth/login" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/login-b.json"
token_b="$(jq -er '.accessToken | select(type == "string" and length > 0)' "$TMP_DIR/login-b.json")"
assert_json "$TMP_DIR/login-b.json" '.user.role == "owner"' "company B login response is invalid"

http 200 "$BASE_URL/api/v1/auth/me" -H "Authorization: Bearer $token_a"
cp "$TMP_DIR/body" "$TMP_DIR/me-a.json"
jq -e --arg slug "$slug_a" '.company.slug == $slug and .user.role == "owner"' "$TMP_DIR/me-a.json" >/dev/null || fail "authenticated identity does not match company A"

payload="$(jq -n --arg slug "smoke-course-$suffix" '{title:"Tenant Isolation Smoke Course",slug:$slug,description:"Automated API smoke test",priceInPaise:0}')"
http 201 "$BASE_URL/api/v1/courses" -H "Authorization: Bearer $token_a" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/course.json"
course_id="$(jq -er '.id | select(type == "string" and length > 0)' "$TMP_DIR/course.json")"
assert_json "$TMP_DIR/course.json" '.status == "draft" and .title == "Tenant Isolation Smoke Course"' "course creation response is invalid"

# A user in company B must not publish company A's course, even when its ID is known.
http 404 "$BASE_URL/api/v1/courses/$course_id/publish" -H "Authorization: Bearer $token_b"
cp "$TMP_DIR/body" "$TMP_DIR/cross-tenant.json"
assert_json "$TMP_DIR/cross-tenant.json" '.error.code == "not_found"' "cross-tenant course access was not blocked"

http 200 "$BASE_URL/api/v1/courses/$course_id/publish" -H "Authorization: Bearer $token_a"
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

payload="$(jq -n --arg slug "$slug_a" --arg email "$learner_email" --arg password "$password" '{companySlug:$slug,name:"Smoke Test Learner",email:$email,password:$password}')"
http 201 "$BASE_URL/api/v1/auth/register-learner" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/learner.json"
learner_token="$(jq -er '.accessToken | select(type == "string" and length > 0)' "$TMP_DIR/learner.json")"
assert_json "$TMP_DIR/learner.json" '.user.role == "learner"' "learner registration response is invalid"

payload="$(jq -n --arg slug "learner-must-not-create-$suffix" '{title:"Learner Must Not Create",slug:$slug,priceInPaise:0}')"
http 403 "$BASE_URL/api/v1/courses" -H "Authorization: Bearer $learner_token" -H 'Content-Type: application/json' --data "$payload"
cp "$TMP_DIR/body" "$TMP_DIR/learner-forbidden.json"
assert_json "$TMP_DIR/learner-forbidden.json" '.error.code == "forbidden"' "learner was allowed to create a course"

printf 'PASS: health, readiness, security headers, auth guard, registration/login, learner RBAC, course lifecycle, and tenant isolation.\n'
