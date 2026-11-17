# Weather spike: provider, request, and host contract

Status: **planning contract only; no code changes, no build, no test run, no Luma launch** (2026-11-15 local / 2026-11-16 UTC). This is preparation for the next representative Stage 6 case, not a decision to ship weather and not permission to enable executable packages. Declarative plugins remain the only enableable runtime.

Implementation update (2026-11-16): the accepted product scope below is now realized as an installed `isolated-ui` example package using a generic host bridge. The fixed native weather command, provider projection, core widget proposals, and local `Settings.weather` fields in this document are superseded by [the runtime record](CUSTOMIZATION-RUNTIME-SPIKE.md), §20.

Read against commit `000aad9` plus uncommitted changes already present in the working tree. Those changes add free widget positioning, `top-left`, `visibility`, and drag controls to `WidgetHost.svelte`, `WidgetSettings.svelte`, `widgets.ts`, `types/index.ts`, `settings.svelte.ts`, and the native customization document. The tree is **not clean** and another agent may still be editing it. Re-read every cited symbol before implementing; line numbers and shapes will drift.

## Accepted product scope

Weather is an example executable plugin, not a built-in Luma weather feature. It must ultimately install, configure, enable, disable, export, update, and remove through the same public package lifecycle as an outside author's plugin. Location and units belong to that plugin's declared settings. Do not add a core `widgets.weather` field, a core weather Settings section, or a weather-specific Home renderer branch for this example.

The current embedded panel and fixed host weather client are development probes only. They do not satisfy the example-plugin milestone. Before calling the example complete, move its UI/assets into an actual package, resolve identity and grants from that installed package, and expose host-mediated network access through a bounded public extension contract. The example must not require private weather-only host hooks. The core-widget integration proposals later in this historical contract are superseded by this scope clarification.

## 1. Evidence labels

| Label | Meaning |
| --- | --- |
| **Provider fact** | Stated in Open-Meteo's official API documentation, terms, or licence, read at the URLs in §11. |
| **Live probe** | A request made from this workstation with `curl.exe` on 2026-11-16 UTC. It was not made by Luma, not through the app's `reqwest` client, and not through the UI. |
| **Static** | Read from the current working tree. It is not runtime proof. |
| **Proposal** | A requirement for this spike. Not accepted implementation detail until it passes. |
| **Not established** | A claim this record deliberately does not make. |

No Luma build, `cargo check`, `cargo test`, `svelte-check`, lint, benchmark, or live UI check was performed. Only the three `curl` probes in §2.4 were executed.

## 2. Provider decision

### 2.1 Selected provider

**Open-Meteo forecast API**, keyless free endpoint at `https://api.open-meteo.com/v1/forecast`.

This choice is deliberately narrow:

- One fixed HTTPS origin and path; no registration, OAuth, account, device flow, or API key on the free tier.
- One request returns current conditions; no second geocoding, gridpoint, or station lookup.
- Global WGS84 coordinates, so a fixed host-owned city list works.
- Official documentation describes the exact query, response, error object, WMO codes, rate limits, licence, and privacy behavior.

Alternatives were not evaluated in depth for this record. The National Weather Service API is US-only and needs a two-step gridpoint lookup; adding it would expand the contract beyond one fixed request. Open-Meteo geocoding is a separate origin and request and is explicitly out of scope (§2.3).

### 2.2 Provider facts

| Fact | Evidence | Consequence |
| --- | --- | --- |
| The forecast endpoint is `https://api.open-meteo.com/v1/forecast`, accepts `latitude` and `longitude` as required WGS84 floating-point values, and returns JSON. | Official API docs, "API Documentation" and parameter table. | The host can build one fixed GET from a host-owned coordinate table. |
| `current` selects current-condition variables. Requesting only `current` returns a `current` object and does not return hourly or daily arrays. | Official API docs, "Current Weather" and "JSON Return Object"; live probe. | The response is small; no pagination or time-series handling is needed. |
| `temperature_unit` accepts `celsius` (default) or `fahrenheit`; `timeformat` accepts `iso8601` (default) or `unixtime`; `timezone` defaults to GMT. | Official API docs, parameter table. | The host can request Fahrenheit directly and epoch seconds without local-time conversion. |
| A successful response contains top-level `latitude`, `longitude`, `elevation`, `generationtime_ms`, `utc_offset_seconds`, `timezone`, `timezone_abbreviation`, `current_units`, and `current`. `current` contains `time`, `interval`, and each requested variable. | Official API docs, "JSON Return Object"; live probe. | The host must project only display fields and must not forward the raw object. |
| A malformed request returns HTTP 400 with `{ "error": true, "reason": "..." }`. | Official API docs, "Errors"; live probe. | The host must not display the provider's `reason`; it can contain internal parsing detail. |
| Weather conditions are numeric WMO codes. Descriptions must be mapped by the client. | Official API docs, "WMO Weather interpretation codes"; live probe returns `weather_code: 3`. | A local code-to-label map is required; unknown codes need a neutral fallback. |
| Free non-commercial use is limited to under 10,000 calls/day, 5,000/hour, and 600/minute, applied per IP address and application. Open-Meteo reserves the right to block misuse. | Official terms, "Non-Commercial Use"; live probe headers show a 600-call, 60-second window. | A single widget with manual refresh is far below the limit, but no polling or retry loop is acceptable. |
| The free API is for non-commercial use. The data is CC BY 4.0. A link is required next to any location where the data is displayed, with the example text "Weather data by Open-Meteo.com" linking to `https://open-meteo.com/`. | Official terms and licence. | The widget must show a real attribution link, and the parent must confirm Luma qualifies as non-commercial. |
| Open-Meteo may collect IP addresses and keeps webserver logs that may contain geographical coordinates; the privacy notice says logs are deleted after 90 days. | Official terms, "Free Non-Commercial API Service" privacy section. | The chosen city is necessarily disclosed to the provider; Luma must not add its own coordinate logging. |

### 2.3 Keyless and licensing constraint

The free endpoint is keyless, but **keyless does not mean unconditional**. The terms limit the free API to non-commercial use. This record does not establish that Luma qualifies. If Luma is distributed with advertising, subscriptions, or other commercial use, the keyless premise fails and a paid key plus a host credential broker would be required. That is a product and legal decision for the parent, not something this spike may assume.

Do **not** use the Open-Meteo geocoding API in B1. It adds a second origin (`geocoding-api.open-meteo.com`), a second request, free-text input, empty results, and GeoNames attribution. The fixed city list in §3.3 is the B1 location mechanism.

### 2.4 Live probes performed

Three `curl.exe` requests from this workstation, 2026-11-16 UTC:

| Probe | Request | Result |
| --- | --- | --- |
| ISO/timezone probe | `GET https://api.open-meteo.com/v1/forecast?latitude=47.6062&longitude=-122.3321&current=temperature_2m,weather_code,is_day&temperature_unit=celsius&timezone=UTC&forecast_days=1` | HTTP 200; `timezone` came back as `GMT` with `utc_offset_seconds: 0`; `current.time` was an ISO string without a zone suffix; `current.interval` was `900`. |
| Current conditions, epoch/Fahrenheit | `GET https://api.open-meteo.com/v1/forecast?latitude=47.6062&longitude=-122.3321&current=temperature_2m,weather_code,is_day&temperature_unit=fahrenheit&timeformat=unixtime` | HTTP 200; `content-type: application/json; charset=utf-8`; `transfer-encoding: chunked`; `x-ratelimit-limit: 600`, `x-ratelimit-window: 60`, `x-ratelimit-cost: 1`, `x-ratelimit-remaining: 599`. Body under 500 bytes. `current.time` was an integer epoch (`1794795300`), `current.interval` was `900`, `temperature_2m` was `46.0`, `weather_code` was `3`, `is_day` was `0`. |
| Invalid variable | Same path with `current=bogus` | HTTP 400; `{ "reason": "...", "error": true }`. |

These probes establish only that the public endpoint answered this workstation. They do **not** establish that Luma's `reqwest` client, TLS stack, proxy environment, or Windows DNS can reach it; that the UI can render the result; or that timeouts, redirects, 429/5xx, oversized bodies, or malformed payloads are handled.

## 3. Request contract

### 3.1 Fixed request

The host builds exactly one GET per load:

```text
GET https://api.open-meteo.com/v1/forecast
  ?latitude=<host-owned latitude, 4 decimals>
  &longitude=<host-owned longitude, 4 decimals>
  &current=temperature_2m,weather_code
  &temperature_unit=<celsius|fahrenheit>
  &timeformat=unixtime
```

| Element | Required value | Rule |
| --- | --- | --- |
| Method | `GET` | No body, no form, no upload. |
| Scheme | `https` | Reject anything else before building. |
| Host | `api.open-meteo.com` | Constant. No redirect host, no CDN, no proxy URL. |
| Port | none | Default 443. |
| Path | `/v1/forecast` | Constant. |
| `latitude`, `longitude` | WGS84, 4 decimal places | Taken only from the host table in §3.3. Never accepted from the frontend. |
| `current` | `temperature_2m,weather_code` | Constant. Do not request hourly, daily, minutely, past days, or models. |
| `temperature_unit` | `celsius` or `fahrenheit` | Only from the validated `units` setting. |
| `timeformat` | `unixtime` | Avoids timezone and ISO parsing ambiguity. |
| `timezone` | omitted | The provider defaults to GMT; epoch values are timezone-independent. |
| `apikey` | omitted | No key on the free tier. A future commercial deployment needs a separate credential contract. |
| `format` | omitted | JSON default. No CSV/XLSX. |
| Any other query key | omitted | No `elevation`, `models`, `forecast_days`, `past_days`, `daily`, `hourly`, `cell_selection`. |

Use `reqwest::Url::parse` on the constant and `query_pairs_mut()` to append the five values. Do not interpolate strings into a URL. This guarantees encoding and keeps the origin/path fixed.

`is_day` is omitted because B1 displays a text condition, not a day/night icon. Add it only if the UI design uses it; it is one more projected field, not a second request.

### 3.2 No arbitrary URL or coordinate input

The frontend must never send:

- a URL, host, path, scheme, port, or redirect target;
- latitude, longitude, elevation, timezone, or model;
- a query string, headers, cookies, or an API key;
- a display label that is used as a URL component.

The only frontend inputs are a location ID from the fixed catalog and a units enum. Unknown IDs and unknown units are rejected before any client is built or any request is sent.

If the parent later wants free-text place search, that is a new contract: it needs a second allowlisted origin, a bounded search query, result selection, empty/error states, and a privacy decision. It must not be smuggled in as "just pass the coordinates".

### 3.3 Host-owned location catalog (proposal)

B1 exposes a fixed list. The user selects one; no automatic geolocation, no IP lookup, no browser locale or region inference, no `navigator.geolocation`, and no free-text input.

| ID | Label | Latitude | Longitude |
| --- | --- | --- | --- |
| `seattle` | Seattle | 47.6062 | -122.3321 |
| `los-angeles` | Los Angeles | 34.0522 | -118.2437 |
| `chicago` | Chicago | 41.8781 | -87.6298 |
| `new-york` | New York | 40.7128 | -74.0060 |
| `toronto` | Toronto | 43.6532 | -79.3832 |
| `london` | London | 51.5074 | -0.1278 |
| `paris` | Paris | 48.8566 | 2.3522 |
| `berlin` | Berlin | 52.5200 | 13.4050 |
| `tokyo` | Tokyo | 35.6762 | 139.6503 |
| `sydney` | Sydney | -33.8688 | 151.2093 |

The frontend needs only `{ id, label }`; the native side needs `{ id, lat, lon }`. Keeping labels out of Rust and coordinates out of TypeScript avoids shipping coordinates to the webview and avoids a second source of display text. The ID list must match in both places; an unknown native ID returns `invalid` and the widget shows the empty/settings state. Add a focused native test for known/unknown IDs; do not test the coordinates as constants.

The catalog is a proposal. The parent may trim it, but B1 should stay a fixed list rather than add search.

## 4. Native command contract

### 4.1 Signature and projection

```rust
#[tauri::command]
pub async fn weather_read(
    location: String,
    units: String,
) -> Result<WeatherReading, WeatherError>
```

```rust
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherReading {
    temperature: f64,
    units: String,        // "c" | "f"
    weather_code: u16,
    observed_at: i64,     // Unix seconds, UTC
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WeatherError {
    code: &'static str,
    message: &'static str,
}
```

The response contains exactly those four reading fields. Do **not** return:

- requested or resolved latitude/longitude, elevation, timezone, UTC offset, generation time, interval, provider unit strings, or raw JSON;
- the provider's `reason` text;
- any request URL, headers, rate-limit headers, IP, or token;
- another location's data or a location history.

`observed_at` is the provider's `current.time` (data validity time), not Luma's fetch time. The UI must label it "Observed", not imply a live measurement. The provider's current data is 15-minutely (`interval: 900`), so minute-level precision is not claimed.

`weather_code` is a WMO code. The UI maps known codes to labels. Unknown but in-range codes render a neutral "Weather" label rather than failing the request.

### 4.2 Bounds

| Bound | Proposal | Enforcement |
| --- | --- | --- |
| Total request time | 5 s | `reqwest::ClientBuilder::timeout(Duration::from_secs(5))`. This covers connect, TLS, headers, and body. |
| Connect time | 3 s | `connect_timeout(Duration::from_secs(3))`. |
| Redirects | 0 | `redirect(reqwest::redirect::Policy::none())`. Treat any 3xx as an error; never follow. |
| Response body | 64 KiB | Read chunks with `Response::chunk()` and reject before appending past the cap. Do not use `Response::json()` or `bytes()` without a cap. |
| Request body | 0 | GET only. |
| In-flight requests | 1 | A command-level `tokio::sync::Mutex<()>` with `try_lock`; a second call returns `busy`. |
| Retries | 0 | No automatic retry. The user may press Refresh/Retry. |
| Cache | 0 | No native cache, no localStorage reading cache, no HTTP validator. |
| Polling | 0 | No interval, visibility timer, network-event listener, or auto-refresh. |
| Auth | 0 | No `Authorization`, cookie store, API key, or credential broker. |
| Logging | 0 location data | Never log the URL (it contains coordinates), the location ID, or the provider body. |

The client must be built once with `OnceLock<reqwest::Client>`. Use `.build().expect("valid weather client")`, **not** `unwrap_or_default()`: a default fallback would silently restore redirect following and lose the timeout policy. The existing commands use `unwrap_or_default()` in places; do not copy that for a policy-bearing client.

The chunk cap is checked before `Vec::extend_from_slice`, but a single malicious chunk could already be allocated by the HTTP stack. The endpoint is fixed and trusted; state this as residual risk, not as a solved memory bound.

### 4.3 Status and error mapping

| Condition | Code | UI behavior |
| --- | --- | --- |
| Unknown location ID or units | `invalid` | Empty/settings state. No request was sent. |
| Another request in flight | `busy` | Recoverable "Weather is still loading." with Retry. |
| Total or body-read timeout | `timeout` | "Weather took too long to load." with Retry. |
| DNS, connect, TLS, proxy, or socket failure | `unavailable` | "Weather is unavailable. Check your connection." with Retry. |
| HTTP 3xx | `unavailable` | Redirect was not followed. No retry loop. |
| HTTP 400/401/403/404 | `unavailable` | A fixed valid query should not produce these; do not display the provider reason. Use the host message "Weather is unavailable. Try again." rather than a connection claim. |
| HTTP 429 | `unavailable` | Do not add a rate-limit countdown or automatic retry. |
| HTTP 5xx | `unavailable` | Retry is user-driven. |
| Body over 64 KiB | `too_large` | "Weather data was too large." with Retry. |
| Invalid JSON, missing `current`, wrong types, non-finite temperature, code outside 0–99, `time <= 0` | `malformed` | "Weather data could not be read." with Retry. |
| Missing location setting | no command | Empty state; no network. |
| Browser dev (`native === false`) | no command | "Weather is available in the desktop app." |

There is no separate `offline` code. The native client cannot reliably distinguish offline from DNS, proxy, or provider failure without a second probe, and this spike must not add one. The `unavailable` message names the connection so the user has an actionable state.

Validate `temperature` as finite and within a broad sanity range (proposal: `-150.0..=150.0`, which covers both units). Rejecting a provider value as malformed is preferable to rendering `NaN` or an absurd number. Ignore unknown JSON fields so provider additions do not break the widget.

### 4.4 Implementation sketch

```rust
const ENDPOINT: &str = "https://api.open-meteo.com/v1/forecast";
const MAX_BODY: usize = 64 * 1024;

fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .connect_timeout(Duration::from_secs(3))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("valid weather client")
    })
}
```

Then: validate `location` and `units`; `try_lock`; build the URL with `query_pairs_mut`; send; require `StatusCode::OK`; read capped chunks; parse into a private `ProviderCurrent` with optional fields; validate and project; return.

Do not accept a `Url` or base URL argument. If a test seam is needed, keep a private `async fn fetch(url: Url)` called only by the command with the constant-built URL; tests may call it with a local test server. The release path must never accept a caller-supplied URL.

## 5. Settings and storage contract

### 5.1 Shape

```ts
export type WeatherLocationId =
  | 'seattle' | 'los-angeles' | 'chicago' | 'new-york' | 'toronto'
  | 'london' | 'paris' | 'berlin' | 'tokyo' | 'sydney';

export type WeatherUnits = 'c' | 'f';

export interface WeatherSettings {
  location: WeatherLocationId | null;
  units: WeatherUnits;
}

export interface WeatherReading {
  temperature: number;
  units: WeatherUnits;
  weatherCode: number;
  observedAt: number;
}
```

`Settings` gains `weather: WeatherSettings`. `WidgetId` gains `'weather'`; `Settings['widgets']` therefore gains `weather: WidgetConfig`.

`WidgetConfig` currently also has `visibility` and `position` in the uncommitted working tree. Weather uses the same shape:

| Field | Default | Meaning |
| --- | --- | --- |
| `enabled` | `false` | Off until the user enables it. |
| `placement` | `bottom-right` | A corner preset. Weather does not use `bar`. |
| `visibility` | `always` | Weather has no playback dependency. |
| `position` | `null` | Host derives the corner position; the user may drag it later. |

If the widget-layout change does not land, drop `visibility` and `position` from this table and use only `enabled`/`placement`. Do not invent fields that the native document does not accept.

### 5.2 Location is local-only

`Settings.weather.location` is personal data. It must not enter:

- the canonical customization document (`src-tauri/src/customization/document.rs`);
- history checkpoints or saved-version restore;
- MCP resources or candidate schemas;
- plugin export;
- logs, error messages, analytics, or notices.

This is achievable with the existing architecture: `customizationConfig()` in `src/lib/services/customization.ts` is a `Pick` of exported fields. `weather` is not in that pick, exactly as `paths` is already excluded. The widget's `enabled`/`placement` remain in `widgets` and therefore in the document; only the location and units stay local.

Consequences to accept:

- Exporting/importing a layout on another machine enables the weather widget but leaves the location unset. The widget shows the empty state until the user chooses a city. This is correct for privacy.
- The web remote and Display Box do not receive the location, because they read the canonical document. B1 does not add a weather remote setting.
- If the parent wants units to be portable, move only `units` into the document later; keep `location` local. Do not add location to the document to make export simpler.

### 5.3 Local persistence

In `src/lib/state/settings.svelte.ts`:

- `defaultSettings()` adds `weather: { location: null, units: 'c' }` and the weather widget config.
- `readSettings()` accepts only a location ID present in the catalog, otherwise `null`; accepts only `'c'`/`'f'`, otherwise `'c'`. An invalid location becomes the empty state, not a silent default city.
- `readSettings()` validates `widgets.weather` with the same rules as Spotify: boolean `enabled`, a corner `placement`, a known `visibility`, and `position` null or integer `x`/`y` from 0 through 1000. Invalid fields fall back to the weather defaults, not a crash.
- `persist()` includes `weather` in the `luma-v1` localStorage object. It is not part of the customization sync.
- The placement validation array gains `top-left` only if the uncommitted widget work has landed; weather placements are `top-right`, `bottom-right`, `bottom-left` (and `top-left` when available). Weather must not accept `bar`.

The location/units controls belong in `WidgetSettings.svelte`, not a new settings category:

- a `<select>` with a "Choose a location…" placeholder that sets `location = null`;
- a units toggle/select (`Celsius`/`Fahrenheit`);
- shown only when the weather widget is enabled, matching the existing pattern of showing placement controls only when enabled;
- "Reset widgets" resets the weather widget's display config but does not silently pick a city; the placeholder is how the user clears a location.

The existing widget visibility select is Spotify-specific ("When Spotify is available", "Only during playback"). Weather must not display those labels. Either extend `WidgetDefinition` with per-widget visibility options (weather: "Always") or suppress the select for weather. The per-widget option is the cleaner fit for the new definition model; see §7.

### 5.4 Canonical document changes

Adding `weather` to `Settings['widgets']` means the native document must accept it. In the current tree:

- `src-tauri/src/customization/document.rs` has `Widgets { spotify: Widget }` with `deny_unknown_fields`. Add `weather: Widget` with a serde default so existing documents without it still load.
- `Default for Config` must include a disabled weather widget.
- `validate` must validate the weather widget's placement, visibility, and position. Factor a small `validate_widget` helper rather than duplicating the Spotify checks.
- The uncommitted `Widget` already has `visibility` and `position`; if that lands, weather inherits them. If it does not, use the committed `Widget` shape.
- `src-tauri/src/customization/mcp.rs` must add `weather` to the candidate schema's `widgets` object and to the capabilities summary. **Static observation:** the current working-tree schema now carries `enabled`, `placement`, `visibility`, and `position` for `spotify`, but no `weather` key. Mirror that shape for weather (with the corner-only placement enum) so MCP commits are not rejected client-side by `additionalProperties: false`.
- `src-tauri/src/customization/plugin.rs` currently contributes only `spotifyEnabled`/`spotifyPlacement` and `copy_field("widgets")` clones the whole `Widgets`. Adding weather does not let a plugin control weather. A plugin that owns `widgets` still owns the whole field; a user change to weather's enabled/placement/position would detach that plugin just like a Spotify change. Decide whether that is acceptable; B1 should not add a weather contribution.

**Ordering:** land the native `Widgets.weather` change before the frontend starts writing it. `customizationConfig()` sends the whole `widgets` object on every save; if the Rust struct still rejects `weather`, every customization commit fails validation and the settings sync reports an error. The frontend and native changes are one slice, not independent.

### 5.5 Document version

Preferred B1: keep document `version: 3` and add `weather` as an additive, serde-defaulted field. Existing v3 documents load with weather disabled. New documents include `weather`. A downgrade to an older Luma would reject the new field because the struct is strict; that is the same forward-compatibility risk as any additive field.

If downgrade compatibility is required, bump to version 4 and add a v3 migration that inserts the disabled weather widget. That is more code and more tests; it is a parent decision, not a weather detail.

## 6. Widget and UI contract

### 6.1 States

| State | Trigger | Display | Action controls | Network |
| --- | --- | --- | --- | --- |
| Loading | Mount or refresh | Card frame, location label if known, "Loading weather…", no fake values | Refresh/Retry remains present but ignores activation; Move remains host-owned | One request |
| Ready | Valid reading | Location, rounded temperature with unit, condition label, "Observed HH:MM", attribution | Refresh, attribution, host Move | None |
| Stale | Refresh failed with a previous reading | Previous values, "Couldn't refresh. Showing HH:MM.", attribution | Retry, attribution, host Move | None until the user retries |
| Empty | No location selected | "Choose a weather location in Settings > Appearance & Home > Widgets." | Host Move only | None |
| Error | Initial load failed | Typed message, no fabricated data | Retry, host Move | None until retry |
| Unavailable | `native === false` | "Weather is available in the desktop app." | Host Move only | None |
| Disabled | `enabled === false` | Not rendered at all | None | None |

There is no "crashed" state for this host-rendered Svelte component. Renderer crash containment is the executable-runtime case, not this widget; do not claim it is covered.

The provider can return a valid response with no `current` object. Treat that as `malformed`, not "empty": empty means the user has not chosen a location. Do not invent a "no data" state from a provider schema failure.

### 6.2 Copy

Keep the copy short enough for a TV card:

- Loading: `Loading weather…`
- Empty: `Choose a weather location in Settings > Appearance & Home > Widgets.`
- Timeout: `Weather took too long to load.`
- Unavailable: `Weather is unavailable. Check your connection.`
- Malformed: `Weather data could not be read.`
- Too large: `Weather data was too large.`
- Busy: `Weather is still loading.`
- Stale: `Couldn't refresh. Showing the last reading from 7:14 PM.`
- Ready: `Seattle · 46°F · Partly cloudy · Observed 7:14 PM`
- Attribution: `Weather data by Open-Meteo.com`

Do not show the provider's raw `reason`, HTTP status, URL, coordinates, or a stack trace.

### 6.3 Attribution

CC BY 4.0 requires a link next to the displayed data. The weather card must therefore include a real link, not only a text credit:

- text: `Weather data by Open-Meteo.com`;
- target: `https://open-meteo.com/`;
- open it in the system browser through a fixed host command, never by navigating the main window;
- no user-supplied URL, no `window.open` with a dynamic target, no remote image.

The smallest host path is to extend the existing fixed-URL `open_setup_link` match in `src-tauri/src/background.rs` with a `"weather"` arm pointing at `https://open-meteo.com/`, then call the existing `openSetupLink('weather')` wrapper. A dedicated `weather_attribution` command is also acceptable but is one more command for no additional safety. Do not add the URL to the CSP or to a new capability.

The link is focusable. That is acceptable because the widget host already adds a Move button and the card is part of the focus order. If the parent insists on a strictly non-focusable display-only widget, the licence's link requirement conflicts; decide before implementation rather than shipping a text-only credit.

### 6.4 Focus and input

- The weather card lives inside the widget host's `.widget-layer` frame. It must be a normal in-flow card: no `position: fixed`, no `inset`, no self-managed corner offsets. The frame owns placement and drag.
- The host's Move button and direction controls are already focusable and handle Escape/Backspace. The weather component must not add global key handlers or intercept Back/Home.
- Keep the Refresh/Retry control as one stable DOM element across loading, ready, stale, and error. Change its label and use `aria-disabled` plus a click guard while a request is in flight. Do not use the `disabled` attribute, which can remove focus and drop the user back to the top of the page.
- Do not auto-focus the card on load. Do not trap focus. Do not remove the attribution link while data is visible.
- `preserveHomeFocus` in `src/lib/services/navigation.ts` tracks `.home-layout`, not `.widget-layer`. A weather re-render while its control is focused is **not** covered by that recovery. Keep the stable control and verify focus on a live D-pad run; do not claim recovery from static reading.
- The phone remote routes navigation to `main`; the weather controls are ordinary focusables in `main`. No remote router change is needed. Verify arrow/activate/Back/Home on a live phone; not run.
- At 1080p and 4K, the card must not obscure its own controls or the host Move button. Not run.

### 6.5 Lifecycle, polling, and cache

- On mount: if `enabled`, native, and `location` is set, issue one request. If not, show empty/unavailable and issue none.
- On location/units change: the settings panel replaces the widget content, so the card unmounts and remounts after Settings closes. There is no live location prop change while a request is in flight in B1. If the parent later keeps the card mounted behind Settings, add a request-generation guard before allowing that.
- Unmount (player open, Settings open, hide-to-tray, disable): increment a component request generation so a late invoke result is ignored. There is no timer or subscription to clear. The native request may still complete within 5 s; its result is discarded.
- Hide-to-tray: `background::hide_to_tray` unloads `main`; the widget is destroyed with the webview. No weather work survives. Do not add a background service.
- Startup: a disabled weather widget issues no request. An enabled widget issues one request after its first mount; the app starts on Home, so this is normally at startup. This is expected user-enabled work, not a background poll.
- No polling, no interval, no `network-changed` subscription, no automatic retry, no on-visibility refresh. The only refresh triggers are mount and the explicit Refresh/Retry action.
- No cache: a remount fetches again. This deliberately accepts redundant requests when the user opens/closes the player or Settings repeatedly. If that becomes a problem, add a bounded host cache as a separate decision; do not smuggle one into B1.
- The last reading retained for the stale state is component view state, not a cache. It dies on unmount.

## 7. Integration points

The working tree currently contains uncommitted widget-layout work. The table describes the current tree, not the committed baseline.

### 7.1 Frontend

| File | Current symbol | Required weather change |
| --- | --- | --- |
| `src/lib/types/index.ts` | `WidgetId`, `WidgetConfig`, `Settings` | Add `'weather'`; add `WeatherLocationId`, `WeatherUnits`, `WeatherSettings`, `WeatherReading`; add `Settings.weather`. |
| `src/lib/services/weather.ts` (new) | none | Fixed `weatherLocations` (`id`, `label`), WMO code-to-label map with neutral fallback, small formatting helpers if needed. No network. |
| `src/lib/services/tauri.ts` | command wrappers | Add `weatherRead(location, units)` invoking `weather_read`. |
| `src/lib/services/widgets.ts` | `WidgetDefinition` | Add the weather definition: name, description, component, corner placements, `isVisible: () => true`. Add per-widget visibility options if the generic Spotify labels are not fixed first. |
| `src/lib/components/WeatherWidget.svelte` (new) | none | State machine, fetch, loading/ready/stale/empty/error, attribution, stable Refresh/Retry. |
| `src/lib/components/WidgetHost.svelte` | `active`, `position`, `onposition` | Pass the weather settings to the weather branch; keep the frame/drag behavior; do not add a Home-only special case unless the product decides widgets are screen-scoped. |
| `src/lib/components/WidgetSettings.svelte` | `reset`, placement buttons, visibility select | Location/units controls; weather reset; per-widget visibility labels. |
| `src/lib/state/settings.svelte.ts` | `defaultSettings`, `readSettings`, `persist` | Add weather defaults, validation, and local persistence. Keep `weather` out of the customization sync. |
| `src/lib/services/customization.ts` | `customizationConfig` | No weather top-level field. `widgets` already flows through; confirm the native document accepts the new key. |
| `src/App.svelte` | `WidgetHost` call | Pass `weather={settings.weather}`; no other routing change. |
| `src/lib/components/HomePreview.svelte`; `src/lib/components/SettingsPanel.svelte` | Spotify-only preview (`widget={settings.widgets.spotify}`) | **Gap:** the layout preview does not show a weather card. Either add a small placeholder card when weather is enabled, or document that the preview is Spotify-only. Do not leave the preview silently implying weather is absent. |
| `src/lib/services/settingsNavigation.ts` | widgets section description | Update "Now playing and future widgets" to mention weather. Minor copy. |

The generic widget component signature in `WidgetDefinition` is `Component<{ placement; onopen }>`; it has no `weather` prop. The smallest plumbing is a weather branch in `WidgetHost` that renders `<WeatherWidget placement={...} weather={weather} />`; do not change `MiniPlayer`'s props or pass unknown props to every widget.

### 7.2 Native

| File | Current symbol | Required weather change |
| --- | --- | --- |
| `src-tauri/src/commands/weather.rs` (new) | none | Fixed endpoint, catalog, client, bounds, projection, typed errors, tests. |
| `src-tauri/src/commands/mod.rs` | module list | Add `pub mod weather;`. |
| `src-tauri/src/lib.rs` | `use commands::{...}`; `generate_handler!` | Import the module and register `weather::weather_read`. No managed state is needed beyond command-local statics. |
| `src-tauri/src/customization/document.rs` | `Widgets`, `Widget`, `validate`, `Default for Config` | Add weather, default disabled, validate its widget fields. |
| `src-tauri/src/customization/mcp.rs` | `candidate_schema`, `capabilities` | Add weather to the widgets schema and summary, mirroring spotify's `enabled`/`placement`/`visibility`/`position` shape with a corner-only placement enum. |
| `src-tauri/src/customization/plugin.rs` | `WidgetContribution`, `copy_field`, `PLACEMENTS` | No weather contribution. Confirm the whole-widgets detach behavior is acceptable; update tests/literals. |
| `src-tauri/src/background.rs` | `hide_to_tray`, `open_setup_link` | No lifecycle change. Add the fixed `"weather"` attribution arm if using the existing opener. |
| `src-tauri/src/extension_spike.rs` | `guard` | No change. The guard already denies non-spike commands on the spike surface, so `weather_read` is not reachable there. A future relay `weather.read` is a separate contract. |
| `src-tauri/src/network.rs` | `network_status`, `watch` | Do not use. No network probe or event subscription. |

### 7.3 Configuration, capabilities, CSP

| Surface | Change |
| --- | --- |
| `src-tauri/tauri.conf.json`, `app.security.csp` | **None.** The request is native. Do not add `https://api.open-meteo.com` to `connect-src`; the frontend must not fetch directly. Do not add image origins; weather uses text labels, not remote icons. |
| `src-tauri/capabilities/default.json` | **None.** Do not add a weather capability. Custom commands are currently default-allowed because there is no build-time command manifest; the extension spike guard blocks the spike surface. This is the existing command-gating gap from `CUSTOMIZATION-RUNTIME-SPIKE.md`, not something weather may fix by widening a capability. |
| `package.json` | No new dependency. `reqwest`, `serde`, `tokio`, `url`, `serde_json`, and `futures-util` already exist. |
| `src-tauri/Cargo.toml` | No new crate or feature. `reqwest` already has `json` and `query`; `Response::chunk` is available without the `stream` feature at the pinned version. |

### 7.4 Test proposals

Only tests that protect meaningful behavior:

- Known/unknown location and units validation: unknown returns `invalid` before a client is built.
- URL builder: exact `https`, `api.open-meteo.com`, `/v1/forecast`, and the five query pairs; no extra keys.
- Projection: valid provider JSON returns exactly four fields; missing `current`, wrong types, non-finite/out-of-range temperature, code outside 0–99, and non-positive `time` return `malformed`.
- Status mapping: 3xx, 400, 429, and 5xx map as in §4.3.
- Frontend `readSettings` invalid location becomes `null`; invalid units becomes `'c'`; `customizationConfig` does not contain `weather`.
- Do not add tests for trivial getters, constants, or the catalog coordinates.

The fixed origin makes failure injection hard without a test seam. A private `fetch(url)` used only by the command and by a local test server is acceptable; a runtime-configurable endpoint is not. If the parent rejects the seam, redirect/oversize/timeout remain live checks.

## 8. Acceptance record

### 8.1 Static checks recorded

| ID | Check | Evidence | Status |
| --- | --- | --- | --- |
| SW1 | Open-Meteo free forecast endpoint is keyless and documents the required query/response. | Official docs; live probe. | Static/provider only |
| SW2 | CC BY 4.0 requires a visible link; free use is non-commercial. | Official terms/licence. | Static; Luma eligibility not established |
| SW3 | Current Luma has no weather provider, weather command, weather settings, or weather widget. | Repository search. | Static only |
| SW4 | Widgets are Spotify-shaped; the current tree is mid-change for free positioning/visibility. | `widgets.ts`, `WidgetHost.svelte`, `types/index.ts`, `document.rs`. | Static only; tree not clean |
| SW5 | The canonical widget document is strict and currently has no weather entry. | `document.rs`, `Widgets`. | Static only |
| SW6 | The MCP candidate schema is strict and currently has no weather entry; `spotify` already carries the new visibility/position fields. | `mcp.rs`, `candidate_schema`. | Static only |
| SW7 | The CSP does not include the weather origin; native `reqwest` bypasses CSP. | `tauri.conf.json`; command patterns. | Static only |
| SW8 | Hide-to-tray unloads `main`; no weather background service is needed. | `background.rs`, `hide_to_tray`. | Static only |
| SW9 | The extension spike guard denies non-spike commands. | `extension_spike.rs`, `guard`. | Static only; live denial not run |

### 8.2 Live checks not run

Every row is **Not run**. No pass/fail result exists.

| ID | Check | Method | Pass condition |
| --- | --- | --- | --- |
| W1 | Happy path | Enable weather, choose a city, observe the card | One request; loading then ready with temperature, condition, observed time, attribution within 5 s. |
| W2 | Empty | Enable weather with no location | No request; clear Settings hint; no spinner. |
| W3 | Offline | Disconnect the network, load/retry | `unavailable` within the timeout; no indefinite spinner; Retry recovers after reconnect. |
| W4 | Timeout | Block or delay the provider (firewall/proxy) | `timeout` at the 5 s bound; the UI is responsive; Retry works. |
| W5 | Redirect denial | Make the provider respond 3xx (controlled proxy/DNS) | No redirect followed; `unavailable`; no second origin contacted. |
| W6 | 429/5xx | Controlled response | `unavailable`; no crash; no automatic retry storm. |
| W7 | Malformed | Controlled body with missing/wrong fields | `malformed`; no fake data; Retry works. |
| W8 | Oversize | Controlled body over 64 KiB | `too_large`; memory stays bounded; no parse of the full body. |
| W9 | One in-flight | Rapid Refresh/Retry | At most one request; second call is `busy` or ignored; UI recovers. |
| W10 | Disable/unmount | Disable, open player, open Settings, hide to tray | No request after unmount; no timer; hide-to-tray destroys the webview; no background work. |
| W11 | Attribution | Activate the link | Opens the system browser at Open-Meteo; the Luma window does not navigate. |
| W12 | Focus/Back | D-pad to Move, Refresh, attribution; press Back/Home; use the phone remote | Controls are reachable; focus does not fall to `body`; Back/Home remain host-owned; the phone routes to `main`. |
| W13 | 1080p/4K | TV check at both resolutions | Text readable at distance; controls not obscured; the card does not cover the host Move button. |
| W14 | Privacy | Inspect `customization_read`, MCP resources, export, history, and logs | No location or coordinates in any of them. |
| W15 | Startup/cost | Start with weather disabled, then enabled | Disabled adds no request; enabled adds one; no polling; no idle network. |
| W16 | Provider reachability from Luma | Run the actual `reqwest` path on Windows | HTTPS/TLS/DNS/proxy work; the probe's curl result is not sufficient. |

## 9. Open decisions

| Item | Why it blocks | Recommendation |
| --- | --- | --- |
| Non-commercial eligibility | The keyless premise depends on it. | Confirm Luma qualifies before implementation. If not, stop and design host-brokered credentials; do not ship a key in the webview. |
| Attribution focus | A real link is required, but it adds a focus stop. | Accept the focusable link; the widget host already adds Move. Do not ship a text-only credit. |
| Fixed list vs free-text location | Free text needs geocoding, a second origin, and empty-result handling. | Keep the fixed list for B1; treat search as a separate contract. |
| Units default | Celsius is the provider default; a US audience may expect Fahrenheit. | Default `'c'` to avoid locale inference; expose the toggle. Change only with a product decision. |
| Local-only weather settings | Keeps location out of the document/export but means it does not travel with a layout. | Keep location and units local in B1; document the empty state after import. |
| Per-widget visibility labels | The current select says "When Spotify is available" for every widget. | Add per-widget visibility options (weather: Always) or suppress the select for weather. |
| Widget overlap | Two corner widgets can be placed on top of each other. | Rely on the new free-position/drag system; default weather to `bottom-right`; verify a default Spotify bar plus weather corner. |
| Document version | An additive field is smaller but breaks downgrade; a bump needs a migration. | Additive v3 with serde default for the spike; bump only if downgrade support is required. |
| Failure-injection seam | A fixed origin makes redirect/oversize tests hard. | Use a private `fetch(url)` test seam; never a runtime endpoint override. |
| Weather icons | A full WMO icon set is new design/assets. | Text condition labels only in B1; icons are a later design task. |
| Global vs Home-only widget | The current widget layer is global; the runtime doc frames weather as a Home widget. | Follow the current global widget behavior in B1; add a screen-scope concept only if the product needs it. |

## 10. What this record does not establish

- That weather is implemented, enabled, shipped, or approved.
- That Open-Meteo is reachable from Luma's `reqwest` client on Windows, or that its TLS/DNS/proxy path works.
- That Luma's distribution qualifies for the free non-commercial API.
- That the CC BY attribution requirement is satisfied by the proposed UI.
- That redirects, timeouts, 429/5xx, oversized bodies, malformed payloads, or offline states are handled; none were exercised.
- That the widget renders correctly at 1080p/4K, that D-pad/phone focus works, or that hide-to-tray cleanup works.
- That the native document/MCP/plugin changes compile or preserve existing documents.
- That any executable package or extension relay may call weather. This is a host-rendered widget contract; executable packages remain disabled and ungranted.
- That the uncommitted widget-layout work will land in its current shape. Re-read the tree.

## 11. Sources

Repository (read-only for this record):

- `AGENTS.md`.
- `docs/CUSTOMIZATION-RUNTIME-SPIKE.md` — weather representative case, relay shape, lifecycle, resource gates.
- `docs/CUSTOMIZATION-LAYERS.md` — Stage 6 scope and network/credential boundaries.
- `docs/PLUGINS.md` — declarative-only runtime and future executable boundary.
- `src/lib/types/index.ts`, `src/lib/services/widgets.ts`, `src/lib/components/WidgetHost.svelte`, `src/lib/components/WidgetSettings.svelte`, `src/lib/components/HomePreview.svelte`, `src/lib/components/SettingsPanel.svelte`, `src/lib/state/settings.svelte.ts`, `src/lib/services/customization.ts`, `src/lib/services/tauri.ts`, `src/lib/services/navigation.ts`, `src/lib/services/settingsNavigation.ts`, `src/App.svelte`.
- `src-tauri/src/commands/{services,steam,sports,youtube}.rs` — current `reqwest` client, timeout, and projection patterns.
- `src-tauri/src/customization/{document,mod,plugin,mcp}.rs` — strict document, widget schema, plugin ownership.
- `src-tauri/src/lib.rs`, `src-tauri/src/background.rs`, `src-tauri/src/extension_spike.rs`, `src-tauri/src/network.rs`.
- `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`, `src-tauri/Cargo.toml`.

Official provider sources:

- [Weather Forecast API](https://open-meteo.com/en/docs) — endpoint, parameters, current variables, JSON response, errors, WMO codes, model table.
- [Terms & Privacy](https://open-meteo.com/en/terms) — non-commercial limits, rate limits, paid tiers, privacy/log retention.
- [Licence](https://open-meteo.com/en/licence) — CC BY 4.0, attribution example, data sources.
- [Geocoding API](https://open-meteo.com/en/docs/geocoding-api) — consulted only to confirm it is a separate origin/request and is out of B1 scope.

Library sources at the pinned versions:

- [reqwest 0.13.1 `Response`](https://docs.rs/reqwest/0.13.1/reqwest/struct.Response.html) — `chunk()`, `content_length()`, `error_for_status()` semantics.
- [reqwest 0.13.1 `redirect::Policy`](https://docs.rs/reqwest/0.13.1/reqwest/redirect/struct.Policy.html) — `Policy::none()`.
- `reqwest::ClientBuilder::timeout` / `connect_timeout` — verify the exact total-time semantics at the pinned version before implementation.

Live probe commands (2026-11-16 UTC):

- `curl.exe -sS -D - "https://api.open-meteo.com/v1/forecast?latitude=47.6062&longitude=-122.3321&current=temperature_2m,weather_code,is_day&temperature_unit=celsius&timezone=UTC&forecast_days=1"`
- `curl.exe -sS -D - "https://api.open-meteo.com/v1/forecast?latitude=47.6062&longitude=-122.3321&current=temperature_2m,weather_code,is_day&temperature_unit=fahrenheit&timeformat=unixtime"`
- `curl.exe -sS -w "\nHTTP %{http_code}\n" "https://api.open-meteo.com/v1/forecast?latitude=47.6062&longitude=-122.3321&current=bogus"`
