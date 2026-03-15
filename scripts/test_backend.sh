#!/usr/bin/env bash
set -euo pipefail

BaseUrl="http://localhost:3000"
AdminUser="admin"
AdminPass="admin123"
VerboseOutput=0

usage() {
  cat <<'USAGE'
Usage: ./scripts/test_backend.sh [options]
  --base-url URL       Base URL (default: http://localhost:3000)
  --admin-user USER    Admin username (default: admin)
  --admin-pass PASS    Admin password (default: admin123)
  --verbose            Verbose request/response output
  --help               Show this help
USAGE
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --base-url)
      BaseUrl="$2"
      shift 2
      ;;
    --admin-user)
      AdminUser="$2"
      shift 2
      ;;
    --admin-pass)
      AdminPass="$2"
      shift 2
      ;;
    --verbose)
      VerboseOutput=1
      shift
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown option: $1"
      usage
      exit 1
      ;;
  esac
done

PYTHON_BIN=""
if command -v python3 >/dev/null 2>&1; then
  PYTHON_BIN="python3"
elif command -v python >/dev/null 2>&1; then
  PYTHON_BIN="python"
else
  echo "python3 or python is required."
  exit 1
fi

write_step() { echo -e "\n=== $1 ==="; }
write_ok() { echo "[OK] $1"; }
write_warn() { echo "[WARN] $1"; }
write_fail() { echo "[FAIL] $1"; }

json_string() {
  "$PYTHON_BIN" - "$1" <<'PY'
import json
import sys
print(json.dumps(sys.argv[1]))
PY
}

json_get() {
  local path="$1"
  "$PYTHON_BIN" - "$path" <<'PY'
import json
import sys

path = sys.argv[1].split(".") if len(sys.argv) > 1 else []
data = json.load(sys.stdin)
for key in path:
    if not key:
        continue
    if isinstance(data, dict):
        data = data.get(key)
    else:
        data = None
        break

if isinstance(data, (dict, list)):
    print(json.dumps(data))
elif data is None:
    print("")
else:
    print(data)
PY
}

json_count() {
  local path="$1"
  "$PYTHON_BIN" - "$path" <<'PY'
import json
import sys

path = sys.argv[1].split(".") if len(sys.argv) > 1 else []
data = json.load(sys.stdin)
for key in path:
    if not key:
        continue
    if isinstance(data, dict):
        data = data.get(key)
    else:
        data = None
        break

if isinstance(data, list):
    print(len(data))
else:
    print(0)
PY
}

invoke_api() {
  local method="$1"
  local path="$2"
  local body="${3:-}"
  local cookie_jar="${4:-}"
  local url
  if [[ "$path" == http* ]]; then
    url="$path"
  else
    url="${BaseUrl}${path}"
  fi

  local curl_args=(-sS -X "$method" "$url")
  if [[ -n "$cookie_jar" ]]; then
    curl_args+=(-b "$cookie_jar" -c "$cookie_jar")
  fi
  if [[ -n "$body" ]]; then
    curl_args+=(-H "Content-Type: application/json" --data "$body")
  fi
  if [[ "$VerboseOutput" -eq 1 ]]; then
    echo "--> $method $url $body"
  fi

  local response
  response="$(curl "${curl_args[@]}" -w "\n%{http_code}")"
  API_STATUS="$(printf '%s' "$response" | tail -n 1)"
  API_BODY="$(printf '%s' "$response" | sed '$d')"

  if [[ "$VerboseOutput" -eq 1 ]]; then
    echo "<-- $API_STATUS $API_BODY"
  fi
}

ADMIN_COOKIE="$(mktemp)"
EMP_COOKIE="$(mktemp)"
cleanup() {
  rm -f "$ADMIN_COOKIE" "$EMP_COOKIE"
}
trap cleanup EXIT

write_step "Auth: Login"
login_body="{\"username\":$(json_string "$AdminUser"),\"password\":$(json_string "$AdminPass")}"
invoke_api "POST" "/api/auth/login" "$login_body" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_fail "Login failed: $API_STATUS $API_BODY"
  exit 1
fi
admin_user="$(printf '%s' "$API_BODY" | json_get "user.username")"
admin_role="$(printf '%s' "$API_BODY" | json_get "user.role")"
write_ok "Logged in as ${admin_user:-unknown} (role=${admin_role:-unknown})"

write_step "Auth: Refresh"
invoke_api "POST" "/api/auth/refresh" "{}" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "refresh failed: $API_STATUS $API_BODY"
else
  write_ok "refresh ok (new tokens issued)"
fi

write_step "Admin: Users list"
invoke_api "GET" "/api/admin/users" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "admin users failed: $API_STATUS $API_BODY"
else
  users_count="$(printf '%s' "$API_BODY" | json_count "")"
  write_ok "users=${users_count}"
fi

write_step "Attendance: Status (pre)"
invoke_api "GET" "/api/attendance/status" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_fail "Status failed: $API_STATUS $API_BODY"
  exit 1
fi
today_status="$(printf '%s' "$API_BODY" | json_get "status")"
attendance_id="$(printf '%s' "$API_BODY" | json_get "attendance_id")"
write_ok "today.status=${today_status}"

write_step "Attendance: Status (today param)"
today="$(date +%F)"
invoke_api "GET" "/api/attendance/status?date=${today}" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "status(date) failed: $API_STATUS $API_BODY"
else
  status_date="$(printf '%s' "$API_BODY" | json_get "status")"
  write_ok "status(date)=${status_date}"
fi

write_step "Attendance: Clock-in"
if [[ "$today_status" == "not_started" ]]; then
  invoke_api "POST" "/api/attendance/clock-in" "{}" "$ADMIN_COOKIE"
  if [[ "$API_STATUS" != 2* ]]; then
    write_fail "clock-in failed: $API_STATUS $API_BODY"
    exit 1
  fi
  attendance_id="$(printf '%s' "$API_BODY" | json_get "id")"
fi
write_ok "attendance_id=${attendance_id}"

if [[ -n "$attendance_id" ]]; then
  write_step "Attendance: Breaks by attendance id"
  invoke_api "GET" "/api/attendance/${attendance_id}/breaks" "" "$ADMIN_COOKIE"
  if [[ "$API_STATUS" != 2* ]]; then
    write_warn "breaks list failed: $API_STATUS $API_BODY"
  else
    breaks_count="$(printf '%s' "$API_BODY" | json_count "")"
    write_ok "breaks=${breaks_count}"
  fi
fi

write_step "Attendance: Break start/end"
invoke_api "GET" "/api/attendance/status" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_fail "Status failed: $API_STATUS $API_BODY"
  exit 1
fi
status_now="$(printf '%s' "$API_BODY" | json_get "status")"
active_break_id="$(printf '%s' "$API_BODY" | json_get "active_break_id")"
if [[ "$status_now" == "clocked_in" ]]; then
  break_start_body="{\"attendance_id\":$(json_string "$attendance_id")}"
  invoke_api "POST" "/api/attendance/break-start" "$break_start_body" "$ADMIN_COOKIE"
  if [[ "$API_STATUS" != 2* ]]; then
    write_warn "break-start failed (continuing): $API_STATUS $API_BODY"
  else
    break_id="$(printf '%s' "$API_BODY" | json_get "id")"
    write_ok "break-start id=${break_id}"
    sleep 1
  fi
fi

invoke_api "GET" "/api/attendance/status" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" == 2* ]]; then
  status_now="$(printf '%s' "$API_BODY" | json_get "status")"
  active_break_id="$(printf '%s' "$API_BODY" | json_get "active_break_id")"
  if [[ "$status_now" == "on_break" && -n "$active_break_id" ]]; then
    break_end_body="{\"break_record_id\":$(json_string "$active_break_id")}"
    invoke_api "POST" "/api/attendance/break-end" "$break_end_body" "$ADMIN_COOKIE"
    if [[ "$API_STATUS" != 2* ]]; then
      write_warn "break-end failed (continuing): $API_STATUS $API_BODY"
    else
      break_end_id="$(printf '%s' "$API_BODY" | json_get "id")"
      write_ok "break-end id=${break_end_id}"
    fi
  fi
fi

write_step "Attendance: Clock-out"
invoke_api "POST" "/api/attendance/clock-out" "{}" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "clock-out failed (continuing): $API_STATUS $API_BODY"
else
  total_hours="$(printf '%s' "$API_BODY" | json_get "total_work_hours")"
  write_ok "clock-out total_hours=${total_hours}"
fi

write_step "Attendance: My list (range today)"
invoke_api "GET" "/api/attendance/me?from=${today}&to=${today}" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "attendance list failed: $API_STATUS $API_BODY"
else
  records_count="$(printf '%s' "$API_BODY" | json_count "")"
  write_ok "records=${records_count}"
fi

write_step "Attendance: Summary (Y/M)"
year="$(date +%Y)"
month="$(date +%-m)"
invoke_api "GET" "/api/attendance/me/summary?year=${year}&month=${month}" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "summary failed: $API_STATUS $API_BODY"
else
  total_work_hours="$(printf '%s' "$API_BODY" | json_get "total_work_hours")"
  total_work_days="$(printf '%s' "$API_BODY" | json_get "total_work_days")"
  write_ok "total=${total_work_hours}, days=${total_work_days}"
fi

write_step "Attendance: My list (Y/M legacy)"
invoke_api "GET" "/api/attendance/me?year=${year}&month=${month}" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "attendance Y/M failed: $API_STATUS $API_BODY"
else
  records_count="$(printf '%s' "$API_BODY" | json_count "")"
  write_ok "records=${records_count}"
fi

write_step "Requests: Create Leave"
start_date="$(date -d "+1 day" +%F)"
end_date="$(date -d "+2 day" +%F)"
leave_body="{\"leave_type\":\"annual\",\"start_date\":\"${start_date}\",\"end_date\":\"${end_date}\",\"reason\":\"test leave\"}"
invoke_api "POST" "/api/requests/leave" "$leave_body" "$ADMIN_COOKIE"
leave_id=""
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "leave create failed: $API_STATUS $API_BODY"
else
  leave_id="$(printf '%s' "$API_BODY" | json_get "id")"
  write_ok "leave id=${leave_id}"
fi

write_step "Requests: Create Overtime"
ot_date="$(date +%F)"
overtime_body="{\"date\":\"${ot_date}\",\"planned_hours\":1.5,\"reason\":\"test overtime\"}"
invoke_api "POST" "/api/requests/overtime" "$overtime_body" "$ADMIN_COOKIE"
overtime_id=""
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "overtime create failed: $API_STATUS $API_BODY"
else
  overtime_id="$(printf '%s' "$API_BODY" | json_get "id")"
  write_ok "overtime id=${overtime_id}"
fi

write_step "Requests: My List"
invoke_api "GET" "/api/requests/me" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "my requests failed: $API_STATUS $API_BODY"
else
  leave_count="$(printf '%s' "$API_BODY" | json_count "leave_requests")"
  overtime_count="$(printf '%s' "$API_BODY" | json_count "overtime_requests")"
  write_ok "leave=${leave_count}, overtime=${overtime_count}"
fi

if [[ -n "$leave_id" ]]; then
  write_step "Admin: Request detail (leave)"
  invoke_api "GET" "/api/admin/requests/${leave_id}" "" "$ADMIN_COOKIE"
  if [[ "$API_STATUS" != 2* ]]; then
    write_warn "admin request detail (leave) failed: $API_STATUS $API_BODY"
  else
    kind="$(printf '%s' "$API_BODY" | json_get "kind")"
    write_ok "kind=${kind}"
  fi
fi

if [[ -n "$overtime_id" ]]; then
  write_step "Admin: Request detail (overtime)"
  invoke_api "GET" "/api/admin/requests/${overtime_id}" "" "$ADMIN_COOKIE"
  if [[ "$API_STATUS" != 2* ]]; then
    write_warn "admin request detail (overtime) failed: $API_STATUS $API_BODY"
  else
    kind="$(printf '%s' "$API_BODY" | json_get "kind")"
    write_ok "kind=${kind}"
  fi
fi

if [[ -n "$leave_id" ]]; then
  write_step "Admin: Approve leave"
  approve_body="{\"comment\":\"looks good\"}"
  invoke_api "PUT" "/api/admin/requests/${leave_id}/approve" "$approve_body" "$ADMIN_COOKIE"
  if [[ "$API_STATUS" != 2* ]]; then
    write_warn "approve failed: $API_STATUS $API_BODY"
  else
    write_ok "approved"
  fi
fi

if [[ -n "$overtime_id" ]]; then
  write_step "Admin: Reject overtime"
  reject_body="{\"comment\":\"no budget\"}"
  invoke_api "PUT" "/api/admin/requests/${overtime_id}/reject" "$reject_body" "$ADMIN_COOKIE"
  if [[ "$API_STATUS" != 2* ]]; then
    write_warn "reject failed: $API_STATUS $API_BODY"
  else
    write_ok "rejected"
  fi
fi

write_step "Admin: List requests (pending)"
invoke_api "GET" "/api/admin/requests?status=pending&page=1&per_page=10" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "admin list failed: $API_STATUS $API_BODY"
else
  leave_count="$(printf '%s' "$API_BODY" | json_count "leave_requests")"
  overtime_count="$(printf '%s' "$API_BODY" | json_count "overtime_requests")"
  write_ok "items: leave=${leave_count}, overtime=${overtime_count}"
fi

write_step "Admin: Attendance list"
invoke_api "GET" "/api/admin/attendance" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "admin attendance failed: $API_STATUS $API_BODY"
else
  count="$(printf '%s' "$API_BODY" | json_count "")"
  write_ok "attendance=${count}"
fi

write_step "Admin: Create non-admin user (employee)"
emp_user="emp_$(date +%Y%m%d%H%M%S)"
emp_pass="Passw0rd!"
create_user_body="{\"username\":\"${emp_user}\",\"password\":\"${emp_pass}\",\"full_name\":\"Test Employee\",\"role\":\"employee\"}"
invoke_api "POST" "/api/admin/users" "$create_user_body" "$ADMIN_COOKIE"
emp_user_id=""
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "create user failed (continuing): $API_STATUS $API_BODY"
else
  emp_user_id="$(printf '%s' "$API_BODY" | json_get "id")"
  write_ok "created ${emp_user}"
fi

write_step "Auth: Login as employee"
emp_login_body="{\"username\":\"${emp_user}\",\"password\":\"${emp_pass}\"}"
invoke_api "POST" "/api/auth/login" "$emp_login_body" "$EMP_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "employee login failed: $API_STATUS $API_BODY"
  emp_logged_in=0
else
  write_ok "employee logged in"
  emp_logged_in=1
fi

if [[ "$emp_logged_in" -eq 1 ]]; then
  write_step "AuthZ: Employee cannot access admin endpoints"
  for entry in "GET /api/admin/users" "GET /api/admin/requests" "GET /api/admin/attendance" "GET /api/admin/export"; do
    method="${entry%% *}"
    path="${entry##* }"
    invoke_api "$method" "$path" "" "$EMP_COOKIE"
    if [[ "$API_STATUS" == 2* ]]; then
      write_fail "expected forbidden but succeeded: $method $path"
    else
      if [[ "$API_STATUS" == "401" || "$API_STATUS" == "403" ]]; then
        write_ok "forbidden as expected: $method $path -> $API_STATUS"
      else
        write_warn "unexpected status for $method $path: $API_STATUS"
      fi
    fi
  done
fi

if [[ "$emp_logged_in" -eq 1 ]]; then
  write_step "Emp Requests: Create/Update/Cancel"
  e_start="$(date -d "+3 day" +%F)"
  e_end="$(date -d "+4 day" +%F)"
  emp_leave_body="{\"leave_type\":\"personal\",\"start_date\":\"${e_start}\",\"end_date\":\"${e_end}\",\"reason\":\"emp leave\"}"
  invoke_api "POST" "/api/requests/leave" "$emp_leave_body" "$EMP_COOKIE"
  if [[ "$API_STATUS" == 2* ]]; then
    emp_leave_id="$(printf '%s' "$API_BODY" | json_get "id")"
    write_ok "emp leave id=${emp_leave_id}"
    emp_update_body="{\"leave_type\":\"personal\",\"start_date\":\"${e_start}\",\"end_date\":\"${e_end}\",\"reason\":\"emp leave (updated)\"}"
    invoke_api "PUT" "/api/requests/${emp_leave_id}" "$emp_update_body" "$EMP_COOKIE"
    if [[ "$API_STATUS" != 2* ]]; then
      write_warn "emp leave update failed: $API_STATUS $API_BODY"
    else
      write_ok "emp leave updated"
    fi
    invoke_api "DELETE" "/api/requests/${emp_leave_id}" "" "$EMP_COOKIE"
    if [[ "$API_STATUS" != 2* ]]; then
      write_warn "emp leave cancel failed: $API_STATUS $API_BODY"
    else
      status="$(printf '%s' "$API_BODY" | json_get "status")"
      write_ok "emp leave cancelled -> ${status}"
    fi
  else
    write_warn "emp leave create failed: $API_STATUS $API_BODY"
  fi
fi

if [[ -n "$emp_user_id" ]]; then
  write_step "Admin: Upsert attendance for employee"
  t="$(date +%F)"
  upsert_body="{\"user_id\":\"${emp_user_id}\",\"date\":\"${t}\",\"clock_in_time\":\"${t}T10:00:00\",\"clock_out_time\":null,\"breaks\":[{\"break_start_time\":\"${t}T12:00:00\",\"break_end_time\":null}]}"
  invoke_api "PUT" "/api/admin/attendance" "$upsert_body" "$ADMIN_COOKIE"
  if [[ "$API_STATUS" == 2* ]]; then
    admin_att_id="$(printf '%s' "$API_BODY" | json_get "id")"
    write_ok "admin upsert attendance_id=${admin_att_id}"
    active_break_id="$(printf '%s' "$API_BODY" | json_get "break_records.0.id")"
    if [[ -n "$active_break_id" ]]; then
      write_step "Admin: Force-end break"
      invoke_api "PUT" "/api/admin/breaks/${active_break_id}/force-end" "{}" "$ADMIN_COOKIE"
      if [[ "$API_STATUS" != 2* ]]; then
        write_warn "force-end failed: $API_STATUS $API_BODY"
      else
        write_ok "force-ended"
      fi
      write_step "Attendance: Breaks (admin upserted)"
      invoke_api "GET" "/api/attendance/${admin_att_id}/breaks" "" "$ADMIN_COOKIE"
      if [[ "$API_STATUS" != 2* ]]; then
        write_warn "breaks(admin) failed: $API_STATUS $API_BODY"
      else
        breaks_count="$(printf '%s' "$API_BODY" | json_count "")"
        write_ok "breaks=${breaks_count}"
      fi
    else
      write_warn "no active break to force-end"
    fi
  else
    write_warn "admin attendance upsert failed: $API_STATUS $API_BODY"
  fi
fi

write_step "Admin: Export data"
invoke_api "GET" "/api/admin/export" "" "$ADMIN_COOKIE"
if [[ "$API_STATUS" != 2* ]]; then
  write_warn "export failed: $API_STATUS $API_BODY"
else
  filename="$(printf '%s' "$API_BODY" | json_get "filename")"
  write_ok "export file=${filename}"
fi

echo -e "\nDone."
