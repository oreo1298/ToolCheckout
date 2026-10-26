// Common Avery label sheets, in millimetres (Letter stocks converted from
// inches). Positions are measured from the top-left corner of the page.

export interface LabelStock {
  id: string;
  name: string;
  page: 'letter' | 'a4';
  pageW: number;
  pageH: number;
  cols: number;
  rows: number;
  labelW: number;
  labelH: number;
  left: number;
  top: number;
  pitchX: number;
  pitchY: number;
  /** Big enough for a QR code, the tool name and the shop name. */
  roomy: boolean;
}

export interface LabelItem {
  tag: string;
  name: string;
  /** Pre-rendered QR code SVG, when QR codes are wanted. */
  qrSvg?: string | null;
}

const IN = 25.4;

export const STOCKS: LabelStock[] = [
  {
    id: 'avery5160',
    name: 'Avery 5160 / 8160 — Letter, 30 per sheet (2⅝″ × 1″)',
    page: 'letter',
    pageW: 8.5 * IN,
    pageH: 11 * IN,
    cols: 3,
    rows: 10,
    labelW: 2.625 * IN,
    labelH: 1 * IN,
    left: 0.1875 * IN,
    top: 0.5 * IN,
    pitchX: 2.75 * IN,
    pitchY: 1 * IN,
    roomy: true,
  },
  {
    id: 'avery5167',
    name: 'Avery 5167 — Letter, 80 per sheet (1¾″ × ½″)',
    page: 'letter',
    pageW: 8.5 * IN,
    pageH: 11 * IN,
    cols: 4,
    rows: 20,
    labelW: 1.75 * IN,
    labelH: 0.5 * IN,
    left: 0.3 * IN,
    top: 0.5 * IN,
    pitchX: 2.05 * IN,
    pitchY: 0.5 * IN,
    roomy: false,
  },
  {
    id: 'avery_l7160',
    name: 'Avery L7160 — A4, 21 per sheet (63.5 × 38.1 mm)',
    page: 'a4',
    pageW: 210,
    pageH: 297,
    cols: 3,
    rows: 7,
    labelW: 63.5,
    labelH: 38.1,
    left: 7.21,
    top: 15.15,
    pitchX: 66.04,
    pitchY: 38.1,
    roomy: true,
  },
  {
    id: 'avery_l7651',
    name: 'Avery L7651 — A4, 65 per sheet (38.1 × 21.2 mm)',
    page: 'a4',
    pageW: 210,
    pageH: 297,
    cols: 5,
    rows: 13,
    labelW: 38.1,
    labelH: 21.2,
    left: 4.75,
    top: 10.7,
    pitchX: 40.6,
    pitchY: 21.2,
    roomy: false,
  },
];

export const stockById = (id: string) => STOCKS.find((s) => s.id === id) ?? STOCKS[0];

export interface Placed<T> {
  item: T;
  page: number;
  x: number;
  y: number;
}

/**
 * Lays items out row by row, page by page. `skip` leaves the first N
 * positions empty so a partly used sheet can be fed again.
 */
export function layout<T>(stock: LabelStock, items: T[], skip = 0): { pages: number; placed: Placed<T>[] } {
  const perPage = stock.cols * stock.rows;
  const start = Math.max(0, Math.min(skip, perPage - 1));
  const placed = items.map((item, i) => {
    const slot = start + i;
    const page = Math.floor(slot / perPage);
    const onPage = slot % perPage;
    const row = Math.floor(onPage / stock.cols);
    const col = onPage % stock.cols;
    return { item, page, x: stock.left + col * stock.pitchX, y: stock.top + row * stock.pitchY };
  });
  const pages = items.length ? placed[placed.length - 1].page + 1 : 0;
  return { pages, placed };
}

/** @page rule for printing on this stock at 100% scale. */
export function pageCss(stock: LabelStock): string {
  return `@page { size: ${stock.page === 'a4' ? 'A4' : 'letter'} portrait; margin: 0; }`;
}
