export function formatSize(bytes){
    if (!bytes) return "0 B";

    const k = 1024;
    const sizes = ["B","KB","MB","GB","TB","PB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    const value = bytes / Math.pow(k, i);

    return `${parseFloat(value.toFixed(2))} ${sizes[i]}`;
}

export function formatTime(seconds) {
    if (!seconds && seconds !== 0) return "0 s";
    
    const h = Math.floor(seconds / 3600);
    const m = Math.floor((seconds % 3600) / 60);
    const s = Math.floor(seconds % 60);

    const parts = [];
    
    if (h > 0) parts.push(`${h} h`);
    if (m > 0) parts.push(`${m} m`);
    if (s > 0 || parts.length === 0) parts.push(`${s} s`);

    return parts.join(" ");
}

export function formatStartTime(startTimeU64) {
  if (!startTimeU64) return "Not started";
  const date = new Date(startTimeU64 * 1000);
  return date.toLocaleString(); 
};