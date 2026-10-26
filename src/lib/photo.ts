// Photo helpers: downscale before upload, cache data URLs for display.
import { api } from './api';

/** Reads an image file and re-encodes it as a JPEG no larger than `max` px. */
export function fileToJpeg(file: File, max = 1280, quality = 0.82): Promise<string> {
  return new Promise((resolve, reject) => {
    const url = URL.createObjectURL(file);
    const img = new Image();
    img.onload = () => {
      URL.revokeObjectURL(url);
      const scale = Math.min(1, max / Math.max(img.naturalWidth, img.naturalHeight));
      const canvas = document.createElement('canvas');
      canvas.width = Math.max(1, Math.round(img.naturalWidth * scale));
      canvas.height = Math.max(1, Math.round(img.naturalHeight * scale));
      const ctx = canvas.getContext('2d');
      if (!ctx) return reject(new Error('Canvas unavailable'));
      ctx.fillStyle = '#fff'; // flatten transparency
      ctx.fillRect(0, 0, canvas.width, canvas.height);
      ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
      resolve(canvas.toDataURL('image/jpeg', quality));
    };
    img.onerror = () => {
      URL.revokeObjectURL(url);
      reject(new Error("That file isn't an image ToolCheckout can read"));
    };
    img.src = url;
  });
}

const cache = new Map<number, Promise<string>>();

export function photoUrl(id: number): Promise<string> {
  let p = cache.get(id);
  if (!p) {
    p = api.getPhoto(id);
    p.catch(() => cache.delete(id));
    cache.set(id, p);
  }
  return p;
}
