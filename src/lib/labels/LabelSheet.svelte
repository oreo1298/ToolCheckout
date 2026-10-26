<script lang="ts">
  // Printable label pages, positioned in millimetres for the chosen stock.
  // Barcodes are Code 128 (what USB scanners read); QR is optional.
  import JsBarcode from 'jsbarcode';
  import { layout, type LabelItem, type LabelStock } from './stocks';

  let {
    stock,
    items,
    skip = 0,
    shopName = '',
    outline = false,
    offsetX = 0,
    offsetY = 0,
  }: {
    stock: LabelStock;
    items: LabelItem[];
    skip?: number;
    shopName?: string;
    outline?: boolean;
    /** Calibration in mm: shifts every label (printers and print engines add margins). */
    offsetX?: number;
    offsetY?: number;
  } = $props();

  // The page box only needs to reach just past the last row/column of labels.
  // Keeping it that small means it fits inside whatever margins the print
  // engine adds (WebKitGTK uses GTK's, which can be uneven), so no blank
  // extra page gets printed.
  const boxW = $derived(stock.left + (stock.cols - 1) * stock.pitchX + stock.labelW + offsetX + 1);
  const boxH = $derived(stock.top + (stock.rows - 1) * stock.pitchY + stock.labelH + offsetY + 1);

  const result = $derived(layout(stock, items, skip));
  const pages = $derived(Array.from({ length: result.pages }, (_, p) => result.placed.filter((x) => x.page === p)));

  /** Renders a Code 128 barcode into an <svg> that scales to its box. */
  function barcode(node: SVGSVGElement, value: string) {
    const draw = (v: string) => {
      JsBarcode(node, v, { format: 'CODE128', displayValue: false, margin: 10, width: 1, height: 40, background: '#ffffff', lineColor: '#000000' });
      const w = node.getAttribute('width');
      const h = node.getAttribute('height');
      node.setAttribute('viewBox', `0 0 ${parseFloat(w ?? '100')} ${parseFloat(h ?? '40')}`);
      node.removeAttribute('width');
      node.removeAttribute('height');
      node.setAttribute('preserveAspectRatio', 'none');
    };
    draw(value);
    return { update: draw };
  }
</script>

{#each pages as page, p (p)}
  <div class="page" style="width:{boxW}mm;height:{boxH}mm">
    {#each page as l, i (i)}
      {@const qr = stock.roomy && l.item.qrSvg}
      <div
        class="label"
        class:outline
        class:roomy={stock.roomy}
        style="left:{l.x + offsetX}mm;top:{l.y + offsetY}mm;width:{stock.labelW}mm;height:{stock.labelH}mm"
      >
        {#if qr}
          <div class="qr" style="width:{stock.labelH - 4}mm;height:{stock.labelH - 4}mm">{@html l.item.qrSvg}</div>
        {/if}
        <div class="text">
          {#if stock.roomy}
            <div class="tag" style="font-size:{Math.min(4.2, stock.labelH / 6)}mm">{l.item.tag}</div>
            <div class="name">{l.item.name}</div>
          {/if}
          <svg class="bc" use:barcode={l.item.tag} style="height:{stock.roomy ? Math.min(11, stock.labelH * 0.38) : stock.labelH * 0.55}mm"></svg>
          {#if !stock.roomy}
            <div class="tag small">{l.item.tag}</div>
          {:else if shopName}
            <div class="shop">{shopName}</div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
{/each}

<style>
  .page {
    position: relative;
    overflow: hidden;
    background: #fff;
    color: #000;
    page-break-after: always;
    break-after: page;
  }
  .page:last-child {
    page-break-after: auto;
    break-after: auto;
  }
  .label {
    position: absolute;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: 1.5mm;
    padding: 1.2mm 2mm;
    overflow: hidden;
    font-family: Arial, Helvetica, sans-serif;
  }
  .label.outline {
    outline: 0.2mm dashed #999;
  }
  .label:not(.roomy) {
    padding: 0.6mm 1.2mm;
  }
  .qr {
    flex: none;
  }
  .qr :global(svg) {
    width: 100%;
    height: 100%;
    display: block;
  }
  .text {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: stretch;
    height: 100%;
  }
  .tag {
    font-weight: 700;
    letter-spacing: 0.04em;
    line-height: 1.1;
  }
  .tag.small {
    font-size: 2.4mm;
    text-align: center;
  }
  .name {
    font-size: 2.6mm;
    line-height: 1.15;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-bottom: 0.6mm;
  }
  .bc {
    width: 100%;
    display: block;
  }
  .shop {
    font-size: 1.9mm;
    color: #333;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-top: 0.4mm;
  }
</style>
