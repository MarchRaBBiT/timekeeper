param(
  [string]$BaseUrl = 'http://localhost:3000',
  [string]$AdminUser = 'admin',
  [string]$AdminPass = 'admin123',
  [switch]$VerboseOutput
)

$ErrorActionPreference = 'Stop'

function Write-Step($msg){ Write-Host "`n=== $msg ===" -ForegroundColor Cyan }
function Write-Ok($msg){ Write-Host "[OK] $msg" -ForegroundColor Green }
function Write-Warn($msg){ Write-Host "[WARN] $msg" -ForegroundColor Yellow }
function Write-Fail($msg){ Write-Host "[FAIL] $msg" -ForegroundColor Red }

function Invoke-Api {
  param(
    [string]$Method,
    [string]$Path,
    [hashtable]$Headers = @{},
    [object]$Body = $null
  )
  $uri = if ($Path.StartsWith('http')) { $Path } else { "$BaseUrl$Path" }
  try {
    $params = @{ Method = $Method; Uri = $uri; Headers = $Headers; ErrorAction = 'Stop' }
    if ($Body -ne $null) {
      $params.ContentType = 'application/json'
      $params.Body = ($Body | ConvertTo-Json -Depth 10)
    }
    if ($VerboseOutput) { Write-Host ("--> {0} {1} {2}" -f $Method, $uri, ($Body | ConvertTo-Json -Depth 10)) -ForegroundColor DarkGray }
    $resp = Invoke-RestMethod @params
    return @{ ok = $true; data = $resp }
  } catch {
    $status = $null
    $text = $null
    $respObj = $null
    try { $respObj = $_.Exception.Response } catch {}
    if ($respObj) {
      try { $status = [int]$respObj.StatusCode } catch {}
      try { $text = $respObj.Content.ReadAsStringAsync().Result } catch {}
    }
    if (-not $text) { $text = $_.ErrorDetails.Message }
    if ($VerboseOutput) { Write-Host ("<-- {0} {1}" -f ($status ?? 'n/a'), $text) -ForegroundColor DarkGray }
    return @{ ok = $false; status = $status; error = $text }
  }
}

# Login as admin
Write-Step "Auth: Login"
$login = Invoke-Api -Method POST -Path '/api/auth/login' -Body @{ username = $AdminUser; password = $AdminPass }
if(-not $login.ok){ Write-Fail "Login failed: $($login.status) $($login.error)"; exit 1 }
$AccessToken = $login.data.access_token
$RefreshToken = $login.data.refresh_token
$AuthHeaders = @{ Authorization = "Bearer $AccessToken" }
Write-Ok "Logged in as $($login.data.user.username) (role=$($login.data.user.role))"

# Refresh token
Write-Step "Auth: Refresh"
$refresh = Invoke-Api -Method POST -Path '/api/auth/refresh' -Body @{ refresh_token = $RefreshToken }
if(-not $refresh.ok){ Write-Warn "refresh failed: $($refresh.status) $($refresh.error)" } else { Write-Ok "refresh ok (new tokens issued)" }

# Admin users list (as admin)
Write-Step "Admin: Users list"
$usersList = Invoke-Api -Method GET -Path '/api/admin/users' -Headers $AuthHeaders
if(-not $usersList.ok){ Write-Warn "admin users failed: $($usersList.status) $($usersList.error)" } else { Write-Ok ("users={0}" -f $usersList.data.Count) }

# Attendance status (pre)
Write-Step "Attendance: Status (pre)"
$status = Invoke-Api -Method GET -Path '/api/attendance/status' -Headers $AuthHeaders
if(-not $status.ok){ Write-Fail "Status failed: $($status.status) $($status.error)"; exit 1 }
$todayStatus = $status.data.status
Write-Ok "today.status=$todayStatus"

# Attendance status with date param
Write-Step "Attendance: Status (today param)"
$today = (Get-Date).ToString('yyyy-MM-dd')
$statusDated = Invoke-Api -Method GET -Path "/api/attendance/status?date=$today" -Headers $AuthHeaders
if(-not $statusDated.ok){ Write-Warn "status(date) failed: $($statusDated.status) $($statusDated.error)" } else { Write-Ok ("status(date)={0}" -f $statusDated.data.status) }

# Ensure clock-in
Write-Step "Attendance: Clock-in"
if($todayStatus -eq 'not_started'){
  $ci = Invoke-Api -Method POST -Path '/api/attendance/clock-in' -Headers $AuthHeaders -Body @{ }
  if(-not $ci.ok){ Write-Fail "clock-in failed: $($ci.status) $($ci.error)"; exit 1 }
  $attendanceId = $ci.data.id
} else {
  # reuse existing attendance id from status
  $attendanceId = $status.data.attendance_id
}
Write-Ok "attendance_id=$attendanceId"

# Break list by attendance id
if($attendanceId){
  Write-Step "Attendance: Breaks by attendance id"
  $brks = Invoke-Api -Method GET -Path "/api/attendance/$attendanceId/breaks" -Headers $AuthHeaders
  if(-not $brks.ok){ Write-Warn "breaks list failed: $($brks.status) $($brks.error)" } else { Write-Ok ("breaks={0}" -f $brks.data.Count) }
}

# Break start and end (if not active)
Write-Step "Attendance: Break start/end"
$status2 = Invoke-Api -Method GET -Path '/api/attendance/status' -Headers $AuthHeaders
if(-not $status2.ok){ Write-Fail "Status failed: $($status2.status) $($status2.error)"; exit 1 }
if($status2.data.status -eq 'clocked_in'){
  $bs = Invoke-Api -Method POST -Path '/api/attendance/break-start' -Headers $AuthHeaders -Body @{ attendance_id = $attendanceId }
  if(-not $bs.ok){ Write-Warn "break-start failed (continuing): $($bs.status) $($bs.error)" } else { Write-Ok "break-start id=$($bs.data.id)"; Start-Sleep -Seconds 1 }
}
$status3 = Invoke-Api -Method GET -Path '/api/attendance/status' -Headers $AuthHeaders
if($status3.ok -and $status3.data.status -eq 'on_break' -and $status3.data.active_break_id){
  $be = Invoke-Api -Method POST -Path '/api/attendance/break-end' -Headers $AuthHeaders -Body @{ break_record_id = $status3.data.active_break_id }
  if(-not $be.ok){ Write-Warn "break-end failed (continuing): $($be.status) $($be.error)" } else { Write-Ok "break-end id=$($be.data.id)" }
}

# Clock-out
Write-Step "Attendance: Clock-out"
$co = Invoke-Api -Method POST -Path '/api/attendance/clock-out' -Headers $AuthHeaders -Body @{ }
if(-not $co.ok){ Write-Warn "clock-out failed (continuing): $($co.status) $($co.error)" } else { Write-Ok "clock-out total_hours=$($co.data.total_work_hours)" }

# Attendance list (range)
Write-Step "Attendance: My list (range today)"
$list = Invoke-Api -Method GET -Path "/api/attendance/me?from=$today&to=$today" -Headers $AuthHeaders
if(-not $list.ok){ Write-Warn "attendance list failed: $($list.status) $($list.error)" } else { Write-Ok ("records={0}" -f $list.data.Count) }

# Summary
Write-Step "Attendance: Summary (Y/M)"
$Y = (Get-Date).Year; $M = (Get-Date).Month
$sum = Invoke-Api -Method GET -Path "/api/attendance/me/summary?year=$Y&month=$M" -Headers $AuthHeaders
if(-not $sum.ok){ Write-Warn "summary failed: $($sum.status) $($sum.error)" } else { Write-Ok ("total={0}, days={1}" -f $sum.data.total_work_hours, $sum.data.total_work_days) }

# Attendance list (legacy Y/M variant)
Write-Step "Attendance: My list (Y/M legacy)"
$listYM = Invoke-Api -Method GET -Path "/api/attendance/me?year=$Y&month=$M" -Headers $AuthHeaders
if(-not $listYM.ok){ Write-Warn "attendance Y/M failed: $($listYM.status) $($listYM.error)" } else { Write-Ok ("records={0}" -f $listYM.data.Count) }

# Create leave request
Write-Step "Requests: Create Leave"
$start = (Get-Date).AddDays(1).ToString('yyyy-MM-dd'); $end = (Get-Date).AddDays(2).ToString('yyyy-MM-dd')
$leave = Invoke-Api -Method POST -Path '/api/requests/leave' -Headers $AuthHeaders -Body @{ leave_type = 'annual'; start_date = $start; end_date = $end; reason = 'test leave' }
if(-not $leave.ok){ Write-Warn "leave create failed: $($leave.status) $($leave.error)" } else { $leaveId = $leave.data.id; Write-Ok "leave id=$leaveId" }

# Create overtime request
Write-Step "Requests: Create Overtime"
$otDate = (Get-Date).ToString('yyyy-MM-dd')
$ot = Invoke-Api -Method POST -Path '/api/requests/overtime' -Headers $AuthHeaders -Body @{ date = $otDate; planned_hours = 1.5; reason = 'test overtime' }
if(-not $ot.ok){ Write-Warn "overtime create failed: $($ot.status) $($ot.error)" } else { $otId = $ot.data.id; Write-Ok "overtime id=$otId" }

# My requests
Write-Step "Requests: My List"
$my = Invoke-Api -Method GET -Path '/api/requests/me' -Headers $AuthHeaders
if(-not $my.ok){ Write-Warn "my requests failed: $($my.status) $($my.error)" } else { Write-Ok ("leave={0}, overtime={1}" -f $my.data.leave_requests.Count, $my.data.overtime_requests.Count) }

# Admin request detail for created requests
if($leaveId){
  Write-Step "Admin: Request detail (leave)"
  $ad1 = Invoke-Api -Method GET -Path "/api/admin/requests/$leaveId" -Headers $AuthHeaders
  if(-not $ad1.ok){ Write-Warn "admin request detail (leave) failed: $($ad1.status) $($ad1.error)" } else { Write-Ok ("kind={0}" -f $ad1.data.kind) }
}
if($otId){
  Write-Step "Admin: Request detail (overtime)"
  $ad2 = Invoke-Api -Method GET -Path "/api/admin/requests/$otId" -Headers $AuthHeaders
  if(-not $ad2.ok){ Write-Warn "admin request detail (overtime) failed: $($ad2.status) $($ad2.error)" } else { Write-Ok ("kind={0}" -f $ad2.data.kind) }
}

# Approve / Reject as admin (using same token)
if($leaveId){
  Write-Step "Admin: Approve leave"
  $ap = Invoke-Api -Method PUT -Path "/api/admin/requests/$leaveId/approve" -Headers $AuthHeaders -Body @{ comment = 'looks good' }
  if(-not $ap.ok){ Write-Warn "approve failed: $($ap.status) $($ap.error)" } else { Write-Ok "approved" }
}
if($otId){
  Write-Step "Admin: Reject overtime"
  $re = Invoke-Api -Method PUT -Path "/api/admin/requests/$otId/reject" -Headers $AuthHeaders -Body @{ comment = 'no budget' }
  if(-not $re.ok){ Write-Warn "reject failed: $($re.status) $($re.error)" } else { Write-Ok "rejected" }
}

# Admin list filtered
Write-Step "Admin: List requests (pending)"
$al = Invoke-Api -Method GET -Path '/api/admin/requests?status=pending&page=1&per_page=10' -Headers $AuthHeaders
if(-not $al.ok){ Write-Warn "admin list failed: $($al.status) $($al.error)" } else { Write-Ok ("items: leave={0}, overtime={1}" -f $al.data.leave_requests.Count, $al.data.overtime_requests.Count) }

# Admin: Attendance list (all)
Write-Step "Admin: Attendance list"
$aa = Invoke-Api -Method GET -Path '/api/admin/attendance' -Headers $AuthHeaders
if(-not $aa.ok){ Write-Warn "admin attendance failed: $($aa.status) $($aa.error)" } else { Write-Ok ("attendance={0}" -f $aa.data.Count) }

# Create a non-admin user and verify admin endpoints are forbidden
Write-Step "Admin: Create non-admin user (employee)"
$EmpUser = 'emp_' + (Get-Date).ToString('yyyyMMddHHmmss')
$EmpPass = 'Passw0rd!'
$createUser = Invoke-Api -Method POST -Path '/api/admin/users' -Headers $AuthHeaders -Body @{ username = $EmpUser; password = $EmpPass; full_name = 'Test Employee'; role = 'employee' }
if(-not $createUser.ok){
  Write-Warn "create user failed (continuing): $($createUser.status) $($createUser.error)"
} else {
  Write-Ok "created $EmpUser"
}

Write-Step "Auth: Login as employee"
$empLogin = Invoke-Api -Method POST -Path '/api/auth/login' -Body @{ username = $EmpUser; password = $EmpPass }
if(-not $empLogin.ok){
  Write-Warn "employee login failed: $($empLogin.status) $($empLogin.error)"
} else {
  $EmpAuthHeaders = @{ Authorization = "Bearer $($empLogin.data.access_token)" }
  Write-Ok "employee logged in"
}

if($empLogin.ok){
  Write-Step "AuthZ: Employee cannot access admin endpoints"
  $forbiddenChecks = @(
    @{ method = 'GET'; path = '/api/admin/users' },
    @{ method = 'GET'; path = '/api/admin/requests' },
    @{ method = 'GET'; path = '/api/admin/attendance' },
    @{ method = 'GET'; path = '/api/admin/export' }
  )
  foreach($chk in $forbiddenChecks){
    $resp = Invoke-Api -Method $chk.method -Path $chk.path -Headers $EmpAuthHeaders
    if($resp.ok){
      Write-Fail ("expected forbidden but succeeded: {0} {1}" -f $chk.method, $chk.path)
    } else {
      if($resp.status -in 401,403){
        Write-Ok ("forbidden as expected: {0} {1} -> {2}" -f $chk.method, $chk.path, $resp.status)
      } else {
        Write-Warn ("unexpected status for {0} {1}: {2}" -f $chk.method, $chk.path, $resp.status)
      }
    }
  }
}

# Employee: Create, update, and cancel own request
if($empLogin.ok){
  Write-Step "Emp Requests: Create/Update/Cancel"
  $eStart = (Get-Date).AddDays(3).ToString('yyyy-MM-dd'); $eEnd = (Get-Date).AddDays(4).ToString('yyyy-MM-dd')
  $empLeave = Invoke-Api -Method POST -Path '/api/requests/leave' -Headers $EmpAuthHeaders -Body @{ leave_type = 'personal'; start_date = $eStart; end_date = $eEnd; reason = 'emp leave' }
  if($empLeave.ok){
    $empLeaveId = $empLeave.data.id
    Write-Ok "emp leave id=$empLeaveId"
    $upd = Invoke-Api -Method PUT -Path "/api/requests/$empLeaveId" -Headers $EmpAuthHeaders -Body @{ leave_type = 'personal'; start_date = $eStart; end_date = $eEnd; reason = 'emp leave (updated)' }
    if(-not $upd.ok){ Write-Warn "emp leave update failed: $($upd.status) $($upd.error)" } else { Write-Ok "emp leave updated" }
    $del = Invoke-Api -Method DELETE -Path "/api/requests/$empLeaveId" -Headers $EmpAuthHeaders
    if(-not $del.ok){ Write-Warn "emp leave cancel failed: $($del.status) $($del.error)" } else { Write-Ok ("emp leave cancelled -> {0}" -f $del.data.status) }
  } else {
    Write-Warn "emp leave create failed: $($empLeave.status) $($empLeave.error)"
  }
}

# Admin: Upsert attendance for employee with active break, then force-end
if($createUser.ok -and $createUser.data.id){
  $EmpUserId = $createUser.data.id
  Write-Step "Admin: Upsert attendance for employee"
  $t = (Get-Date).ToString('yyyy-MM-dd')
  $up = Invoke-Api -Method PUT -Path '/api/admin/attendance' -Headers $AuthHeaders -Body @{
    user_id = $EmpUserId
    date = $t
    clock_in_time = "$t" + 'T10:00:00'
    clock_out_time = $null
    breaks = @(@{ break_start_time = "$t" + 'T12:00:00'; break_end_time = $null })
  }
  if($up.ok){
    $admAttId = $up.data.id
    Write-Ok "admin upsert attendance_id=$admAttId"
    $active = $up.data.break_records | Where-Object { -not $_.break_end_time } | Select-Object -First 1
    if($active){
      Write-Step "Admin: Force-end break"
      $fe = Invoke-Api -Method PUT -Path "/api/admin/breaks/$($active.id)/force-end" -Headers $AuthHeaders
      if(-not $fe.ok){ Write-Warn "force-end failed: $($fe.status) $($fe.error)" } else { Write-Ok "force-ended" }
      Write-Step "Attendance: Breaks (admin upserted)"
      $br2 = Invoke-Api -Method GET -Path "/api/attendance/$admAttId/breaks" -Headers $AuthHeaders
      if(-not $br2.ok){ Write-Warn "breaks(admin) failed: $($br2.status) $($br2.error)" } else { Write-Ok ("breaks={0}" -f $br2.data.Count) }
    } else {
      Write-Warn "no active break to force-end"
    }
  } else {
    Write-Warn "admin attendance upsert failed: $($up.status) $($up.error)"
  }
}

# Admin: Export data
Write-Step "Admin: Export data"
$exp = Invoke-Api -Method GET -Path '/api/admin/export' -Headers $AuthHeaders
if(-not $exp.ok){ Write-Warn "export failed: $($exp.status) $($exp.error)" } else { Write-Ok ("export file={0}" -f $exp.data.filename) }

Write-Host "`nDone." -ForegroundColor Cyan
