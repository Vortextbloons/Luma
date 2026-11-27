const invoke = (command, args) => window.__TAURI_INTERNALS__.invoke(command, args);
const bridge = (op, args) =>
  args === undefined ? invoke("extension_bridge", { op }) : invoke("extension_bridge", { op, args });

const ORIGINS = [
  "https://api.elections.kalshi.com",
  "https://external-api.kalshi.com",
];

const SERIES = [
  { league: "NBA", ticker: "KXNBAGAME" },
  { league: "NFL", ticker: "KXNFLGAME" },
  { league: "MLB", ticker: "KXMLBGAME" },
  { league: "NHL", ticker: "KXNHLGAME" },
  { league: "NCAAB", ticker: "KXNCAABGAME" },
  { league: "WNBA", ticker: "KXWNBAGAME" },
];

let settings = {
  showCardOdds: true,
  showPlayerWidget: true,
  refreshSeconds: "45",
};
let timer = null;
let running = false;

function toCents(dollars, centsFallback) {
  if (dollars !== undefined && dollars !== null && dollars !== "") {
    const parsed = Number(dollars);
    if (Number.isFinite(parsed) && parsed > 0 && parsed <= 1) return parsed * 100;
  }
  const fallback = Number(centsFallback);
  if (Number.isFinite(fallback) && fallback > 0 && fallback <= 100) return fallback;
  return null;
}

function marketPct(market) {
  if (!market || typeof market !== "object") return null;
  const bid = toCents(market.yes_bid_dollars, market.yes_bid);
  const ask = toCents(market.yes_ask_dollars, market.yes_ask);
  const last = toCents(market.last_price_dollars, market.last_price);
  let cents = null;
  if (bid !== null && ask !== null) {
    cents = (bid + ask) / 2;
  } else if (last !== null) {
    cents = last;
  } else if (ask !== null) {
    cents = ask;
  } else if (bid !== null) {
    cents = bid;
  }
  if (cents === null) return null;
  return Math.max(1, Math.min(99, Math.round(cents)));
}

function teamLabel(market) {
  const raw = String(market.yes_sub_title || market.subtitle || market.title || "").trim();
  return raw.replace(/\s+Winner$/i, "").trim();
}

function parseEvents(league, payload) {
  const events = Array.isArray(payload?.events) ? payload.events : [];
  const badges = [];
  for (const event of events) {
    const markets = Array.isArray(event?.markets) ? event.markets : [];
    const sides = [];
    for (const market of markets) {
      const pct = marketPct(market);
      const team = teamLabel(market);
      if (pct !== null && team) {
        sides.push({ team, value: `${pct}%`, pct });
      }
    }
    if (sides.length === 2) {
      const total = sides[0].pct + sides[1].pct;
      if (total > 0 && Math.abs(total - 100) <= 8) {
        const first = Math.round((sides[0].pct / total) * 100);
        sides[0].pct = first;
        sides[0].value = `${first}%`;
        sides[1].pct = 100 - first;
        sides[1].value = `${100 - first}%`;
      }
      badges.push({
        league,
        teams: [sides[0].team, sides[1].team],
        label: "Kalshi",
        sides,
        updatedAt: Math.floor(Date.now() / 1000),
      });
    }
  }
  return badges;
}

async function fetchSeries(seriesTicker) {
  let lastError = null;
  for (const origin of ORIGINS) {
    try {
      const response = await bridge("net.get", {
        origin,
        path: "/trade-api/v2/events",
        query: [
          ["status", "open"],
          ["series_ticker", seriesTicker],
          ["with_nested_markets", "true"],
          ["limit", "25"],
        ],
      });
      if (response && typeof response === "object") {
        return response;
      }
    } catch (error) {
      lastError = error;
    }
  }
  if (lastError) throw lastError;
  return null;
}

async function refresh() {
  if (running) return;
  if (!settings.showCardOdds && !settings.showPlayerWidget) {
    await bridge("sports.badges.publish", {
      badges: [],
      showOnCards: false,
      showInPlayer: false,
    }).catch(() => {});
    return;
  }
  running = true;
  try {
    let activeLeague = null;
    try {
      const active = await bridge("sports.active.read");
      if (active?.watching && typeof active.league === "string") {
        activeLeague = active.league;
      }
    } catch {}

    const targetSeries = activeLeague
      ? SERIES.filter(item => item.league.toLowerCase() === activeLeague.toLowerCase())
      : SERIES;
    const list = targetSeries.length ? targetSeries : SERIES;

    const badges = [];
    for (const item of list) {
      try {
        const data = await fetchSeries(item.ticker);
        badges.push(...parseEvents(item.league, data));
      } catch {}
    }

    await bridge("sports.badges.publish", {
      badges: badges.slice(0, 400),
      showOnCards: Boolean(settings.showCardOdds),
      showInPlayer: Boolean(settings.showPlayerWidget),
    }).catch(() => {});
  } finally {
    running = false;
  }
}

function schedule() {
  if (timer) clearInterval(timer);
  const seconds = Math.max(15, Number(settings.refreshSeconds) || 45);
  timer = setInterval(() => {
    void refresh();
  }, seconds * 1000);
}

async function init() {
  await bridge("surface.ready").catch(() => {});
  try {
    const initial = await bridge("settings.subscribe");
    if (initial?.values && typeof initial.values === "object") {
      settings = { ...settings, ...initial.values };
    }
  } catch {
    try {
      const initial = await bridge("settings.read");
      if (initial?.values && typeof initial.values === "object") {
        settings = { ...settings, ...initial.values };
      }
    } catch {}
  }

  window.addEventListener("luma:settings", event => {
    if (event?.detail?.values && typeof event.detail.values === "object") {
      settings = { ...settings, ...event.detail.values };
      schedule();
      void refresh();
    }
  });

  schedule();
  await refresh();
}

void init();
