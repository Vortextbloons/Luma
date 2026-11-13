import type { MediaItem } from './types';
import hero from '../assets/art-0.webp';
import red from '../assets/art-1.webp';
import echo from '../assets/art-2.webp';
import glass from '../assets/art-3.webp';
import blue from '../assets/art-4.webp';
import regiment from '../assets/art-5.webp';
import midnight from '../assets/art-6.webp';
import tide from '../assets/art-7.webp';
import ember from '../assets/art-8.webp';
import north from '../assets/art-9.webp';
import citadel from '../assets/art-10.webp';

export const heroArtwork = hero;
export const continueWatching: MediaItem[] = [
  { title: 'Red Frontier', subtitle: 'Movie · 58 min left', artwork: red, progress: 41 },
  { title: 'Echo Valley', subtitle: 'S1 E4 · 22 min left', artwork: echo, progress: 72 },
  { title: 'Glass City', subtitle: 'S1 E7 · 46 min left', artwork: glass, progress: 22 },
  { title: 'Blue Planet', subtitle: 'S1 E1 · 48 min left', artwork: blue, progress: 53 },
  { title: 'The Last Regiment', subtitle: 'Movie · 1h 08m left', artwork: regiment, progress: 31 },
];
export const featured: MediaItem[] = [
  { title: 'Midnight Horizon', subtitle: 'Sci-fi · 2025', artwork: midnight },
  { title: 'The Silent Tide', subtitle: 'Documentary · 2024', artwork: tide },
  { title: 'Ember Road', subtitle: 'Drama · 2025', artwork: ember },
  { title: 'Northstar', subtitle: 'Adventure · 2025', artwork: north },
  { title: 'The Citadel', subtitle: 'Fantasy · 2025', artwork: citadel },
];
