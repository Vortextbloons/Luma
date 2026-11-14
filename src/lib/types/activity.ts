export type ActivityType = 'movie' | 'episode' | 'video' | 'track' | 'application';
export type ProgressSource = 'provider' | 'luma' | 'import' | 'unknown';
export type ResumeKind = 'direct' | 'content' | 'service' | 'none';
export type ProviderState = 'available' | 'needsSetup' | 'notValidated' | 'connected' | 'disabled' | 'error';

export interface ActivityItem {
  id: string;
  provider: string;
  providerItemId: string | null;
  profileId: string | null;
  type: ActivityType;
  title: string;
  subtitle: string | null;
  artwork: string | null;
  lastActivityAt: string;
  progressSeconds: number | null;
  durationSeconds: number | null;
  progressSource: ProgressSource;
  completed: boolean | null;
  canResume: boolean;
  launchTarget: string | null;
  resumeKind: ResumeKind;
  launchCount: number | null;
}

export interface ProviderStatus {
  id: string;
  name: string;
  state: ProviderState;
  detail: string;
  enabled: boolean;
  hidden: boolean;
  configured: boolean;
  validated: boolean;
  requiresConfig: boolean;
  capabilities: string[];
  lastSuccessAt: string | null;
  lastError: string | null;
}

export interface ActivitySnapshot {
  items: ActivityItem[];
  providers: ProviderStatus[];
  trackingEnabled: boolean;
  generatedAt: string;
}

export interface ProviderConfigInput {
  serverUrl?: string;
  userId?: string;
  clientId?: string;
  token?: string;
}

export interface RefreshResult {
  snapshot: ActivitySnapshot;
  updated: number;
  error: string | null;
}

export interface OpenOutcome {
  kind: ResumeKind;
  message: string;
}
