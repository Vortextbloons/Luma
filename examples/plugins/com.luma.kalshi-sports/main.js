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
  { league: "CFB", ticker: "KXNCAAFGAME" },
  { league: "NCAA Men", ticker: "KXNCAAMBGAME" },
  { league: "NCAA Men", ticker: "KXNCAABGAME" },
  { league: "NCAA Women", ticker: "KXNCAAWBGAME" },
  { league: "WNBA", ticker: "KXWNBAGAME" },
];

let settings = {
  trackedLeague: "Auto",
  trackedTeam: "",
  spoilerFree: false,
  showWidgetOdds: true,
  showCardOdds: true,
  showPlayerWidget: true,
  refreshSeconds: "45",
};
let timer = null;
let running = false;

function toCents(dollars, centsFallback) {
  if (dollars !== undefined && dollars !== null && dollars !== "") {
    const parsed = Number(dollars);
    if (Number.isFinite(parsed) && parsed >= 0 && parsed <= 1) return parsed * 100;
  }
  const fallback = Number(centsFallback);
  if (Number.isFinite(fallback) && fallback >= 0 && fallback <= 100) return fallback;
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
    if (sides.length === 1 && markets.length === 2) {
      const otherMarket = markets.find(m => teamLabel(m) && teamLabel(m) !== sides[0].team);
      if (otherMarket) {
        const otherTeam = teamLabel(otherMarket);
        const otherPct = Math.max(1, Math.min(99, 100 - sides[0].pct));
        sides.push({ team: otherTeam, value: `${otherPct}%`, pct: otherPct });
      }
    }
    if (sides.length === 2) {
      const total = sides[0].pct + sides[1].pct;
      if (total > 0 && Math.abs(total - 100) <= 15) {
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
          ["limit", "100"],
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

function matchLeague(seriesLeague, targetLeague) {
  if (!targetLeague) return true;
  const s = seriesLeague.toLowerCase();
  const t = targetLeague.toLowerCase();
  if (s === t) return true;
  if ((s === "ncaa men" || s === "ncaab") && (t === "ncaa men" || t === "ncaab")) return true;
  if ((s === "cfb" || s === "ncaaf") && (t === "cfb" || t === "ncaaf")) return true;
  return false;
}

function normalizeTeam(name) {
  return String(name || "")
    .toLowerCase()
    .replace(/\blos angeles\b/g, "la")
    .replace(/\bsaint\b/g, "st")
    .replace(/[^a-z0-9]/g, "");
}

function parseGamesFromScoreboard(payload, league) {
  const events = Array.isArray(payload?.events) ? payload.events : [];
  const games = [];
  for (const event of events) {
    const comp = event?.competitions?.[0];
    if (!comp || !Array.isArray(comp.competitors) || comp.competitors.length !== 2) continue;
    const status = comp.status || event.status || {};
    const state = status?.type?.state || "pre";
    const detail = status?.type?.shortDetail || status?.type?.detail || "";
    const teams = comp.competitors.map(c => ({
      name: c?.team?.displayName || c?.team?.shortDisplayName || "",
      abbr: c?.team?.abbreviation || "",
      score: c?.score == null ? "" : String(c.score),
      record: c?.records?.find(r => r.type === "total")?.summary || "",
      homeAway: c?.homeAway || "",
    }));
    games.push({ id: String(event.id || ""), league, state, detail, teams });
  }
  return games;
}

function pickTrackedGame(games, teamQuery) {
  if (!games.length) return null;
  const query = normalizeTeam(teamQuery);
  if (query) {
    const matching = games.filter(g =>
      g.teams.some(t => normalizeTeam(t.name).includes(query) || normalizeTeam(t.abbr) === query)
    );
    if (matching.length) {
      return matching.find(g => g.state === "in") || matching.find(g => g.state === "pre") || matching[0];
    }
  }
  const live = games.filter(g => g.state === "in");
  if (live.length) {
    live.sort((a, b) => {
      const diffA = Math.abs(Number(a.teams[0]?.score || 0) - Number(a.teams[1]?.score || 0));
      const diffB = Math.abs(Number(b.teams[0]?.score || 0) - Number(b.teams[1]?.score || 0));
      return diffA - diffB;
    });
    return live[0];
  }
  return games.find(g => g.state === "pre") || games[0];
}

function findMatchingBadge(game, badges) {
  if (!game || !badges.length) return null;
  const gNames = game.teams.map(t => normalizeTeam(t.name));
  return badges.find(b => {
    if (!matchLeague(b.league, game.league)) return false;
    const bNames = (b.teams || []).map(normalizeTeam);
    return gNames.every(gn => bNames.some(bn => bn && (bn.includes(gn) || gn.includes(bn))));
  }) || null;
}

async function publishScoreboardWidget(badges, active) {
  const configuredLeague = settings.trackedLeague || "Auto";
  const leaguesToCheck = configuredLeague === "Auto"
    ? (active?.watching && active.league ? [active.league, "NBA", "NFL", "MLB", "NHL"] : ["NBA", "NFL", "MLB", "NHL"])
    : [configuredLeague];
  const uniqueLeagues = [...new Set(leaguesToCheck)];

  let games = [];
  for (const league of uniqueLeagues) {
    try {
      const scoreboard = await bridge("sports.scores.read", { league });
      games.push(...parseGamesFromScoreboard(scoreboard, league));
    } catch {}
  }

  const teamFilter = settings.trackedTeam || (active?.watching && active.title ? active.title : "");
  const game = pickTrackedGame(games, teamFilter);
  if (!game) {
    const label = configuredLeague === "Auto" ? "Sports" : configuredLeague;
    await bridge("widgets.publish", {
      widgets: [{
        id: "live-scoreboard",
        available: Boolean(settings.trackedTeam),
        title: settings.trackedTeam ? `No ${label} game found` : `${label} Scoreboard`,
        subtitle: settings.trackedTeam ? `Tracking "${settings.trackedTeam}"` : "No games scheduled right now",
        tone: "muted",
        updatedAt: Math.floor(Date.now() / 1000),
      }],
    }).catch(() => {});
    return;
  }

  const away = game.teams.find(t => t.homeAway === "away") || game.teams[0];
  const home = game.teams.find(t => t.homeAway === "home") || game.teams[1];
  const awayLabel = away.abbr || away.name;
  const homeLabel = home.abbr || home.name;
  const hidden = Boolean(settings.spoilerFree) && game.state !== "pre";
  const badge = settings.showWidgetOdds ? findMatchingBadge(game, badges) : null;

  let progress;
  let progressLabel;
  let badgeText = game.state === "in" ? "LIVE" : game.state === "post" ? "FINAL" : "UPCOMING";
  if (badge?.sides?.length === 2) {
    const first = badge.sides[0];
    const second = badge.sides[1];
    progress = first.pct;
    progressLabel = `${first.team} ${first.value} · ${second.team} ${second.value}`;
    if (game.state === "in") badgeText = `LIVE · ${first.value}`;
  }

  const meta = [];
  if (hidden) {
    meta.push("Spoiler-free");
  } else if (game.state === "in") {
    const diff = Math.abs(Number(away.score || 0) - Number(home.score || 0));
    const detailLower = game.detail.toLowerCase();
    if (detailLower.includes("ot")) meta.push("Overtime");
    else if (diff <= 5 && (detailLower.includes("4th") || detailLower.includes("3rd") || detailLower.includes("9th") || detailLower.includes("final"))) {
      meta.push("Close game");
    }
  }

  const rows = [away, home].map(team => ({
    label: team.abbr || team.name,
    value: hidden ? "••" : (game.state === "pre" ? (team.record || "—") : (team.score || "0")),
    sub: game.state === "pre" ? undefined : (team.record || undefined),
  }));

  await bridge("widgets.publish", {
    widgets: [{
      id: "live-scoreboard",
      available: true,
      title: `${awayLabel} @ ${homeLabel}`,
      subtitle: `${game.league} · ${game.detail || "Scheduled"}`,
      primary: hidden ? "Score hidden" : (game.state === "pre" ? undefined : `${away.score || 0} – ${home.score || 0}`),
      badge: badgeText,
      tone: game.state === "in" ? "live" : game.state === "post" ? "muted" : "accent",
      progress,
      progressLabel,
      rows,
      meta,
      updatedAt: Math.floor(Date.now() / 1000),
    }],
  }).catch(() => {});
}

async function refresh() {
  if (running) return;
  running = true;
  try {
    let active = null;
    let activeLeague = null;
    try {
      active = await bridge("sports.active.read");
      if (active?.watching && typeof active.league === "string") {
        activeLeague = active.league;
      }
    } catch {}

    let uniqueBadges = [];
    if (settings.showCardOdds || settings.showPlayerWidget || settings.showWidgetOdds) {
      const targetSeries = activeLeague
        ? SERIES.filter(item => matchLeague(item.league, activeLeague))
        : SERIES;
      const list = targetSeries.length ? targetSeries : SERIES;

      const badges = [];
      for (const item of list) {
        try {
          const data = await fetchSeries(item.ticker);
          badges.push(...parseEvents(item.league, data));
        } catch {}
      }

      const seen = new Set();
      for (const badge of badges) {
        const key = `${badge.league}:${[...badge.teams].sort().join(" vs ")}`;
        if (!seen.has(key)) {
          seen.add(key);
          uniqueBadges.push(badge);
        }
      }
    }

    await bridge("sports.badges.publish", {
      badges: (settings.showCardOdds || settings.showPlayerWidget) ? uniqueBadges.slice(0, 400) : [],
      showOnCards: Boolean(settings.showCardOdds),
      showInPlayer: Boolean(settings.showPlayerWidget),
    }).catch(() => {});

    await publishScoreboardWidget(uniqueBadges, active);
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
