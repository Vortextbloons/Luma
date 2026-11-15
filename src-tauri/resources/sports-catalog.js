(() => {
  if (window.top !== window || location.hostname !== 'gostreameast.one') return;
  Object.defineProperty(window, '__lx', {
    configurable: true,
    set(load) {
      const requests = new Map();
      const cached = index => {
        if (!requests.has(index)) requests.set(index, load(index));
        return requests.get(index);
      };
      Object.defineProperty(window, '__lx', { value: cached, configurable: true });
      cached(0).then(data => {
        if (!Array.isArray(data.matches)) throw new Error();
        const matches = data.matches.filter(match => !match.finished).map(match => ({
          id: match.id, api_id: match.api_id, title: match.title, sport_name: match.sport_name,
          starts_at: match.starts_at, live: match.live, home: match.home, away: match.away,
          streams: (match.streams || []).map(stream => ({ url: stream.embed_url })),
        }));
        document.title = 'LumaSportsCatalog:' + JSON.stringify({ matches });
      }).catch(() => { document.title = 'LumaSportsCatalog:{}'; });
    },
  });
})();
