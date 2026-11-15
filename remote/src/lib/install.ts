export interface InstallEnvironment {
  userAgent: string;
  platform: string;
  maxTouchPoints: number;
  standalone: boolean;
  hostname: string;
}

export function isStandalone(): boolean {
  if (typeof window === "undefined") return false;
  const media = typeof window.matchMedia === "function" && window.matchMedia("(display-mode: standalone)").matches;
  const ios = (navigator as Navigator & { standalone?: boolean }).standalone === true;
  return Boolean(media || ios);
}

export function installEnvironment(): InstallEnvironment {
  return {
    userAgent: typeof navigator === "undefined" ? "" : navigator.userAgent,
    platform: typeof navigator === "undefined" ? "" : navigator.platform,
    maxTouchPoints: typeof navigator === "undefined" ? 0 : navigator.maxTouchPoints ?? 0,
    standalone: isStandalone(),
    hostname: typeof location === "undefined" ? "" : location.hostname,
  };
}

function isAppleMobile(environment: InstallEnvironment): boolean {
  return /iPad|iPhone|iPod/.test(environment.userAgent)
    || (environment.platform === "MacIntel" && environment.maxTouchPoints > 1);
}

export function installSteps(environment: InstallEnvironment): string {
  if (isAppleMobile(environment)) {
    return "Tap Share, then Add to Home Screen. Keep Open as Web App on if it appears.";
  }
  if (/Android/.test(environment.userAgent)) {
    return "Open the browser menu, then tap Add to Home screen.";
  }
  return "Use your browser menu to add this page to your home screen.";
}

export function offerInstall(environment: InstallEnvironment): boolean {
  return !environment.standalone
    && environment.hostname !== "localhost"
    && environment.hostname !== "127.0.0.1";
}
