<script lang="ts">
  // Clean printable report (use "Save as PDF" in the print dialog for a PDF).
  let {
    title,
    subtitle = '',
    shopName = '',
    headers,
    rows,
    numeric = [],
  }: {
    title: string;
    subtitle?: string;
    shopName?: string;
    headers: string[];
    rows: string[][];
    numeric?: number[];
  } = $props();
  const generated = new Date().toLocaleString();
</script>

<article class="report">
  <header>
    <div class="shop">{shopName}</div>
    <h1>{title}</h1>
    {#if subtitle}<div class="sub">{subtitle}</div>{/if}
    <div class="gen">Generated {generated} · {rows.length} rows</div>
  </header>
  <table>
    <thead>
      <tr>{#each headers as h, i (i)}<th class:num={numeric.includes(i)}>{h}</th>{/each}</tr>
    </thead>
    <tbody>
      {#each rows as r, ri (ri)}
        <tr>{#each r as c, i (i)}<td class:num={numeric.includes(i)}>{c}</td>{/each}</tr>
      {:else}
        <tr><td colspan={headers.length}>No data for this selection.</td></tr>
      {/each}
    </tbody>
  </table>
</article>

<style>
  .report {
    font-family: Arial, Helvetica, sans-serif;
    color: #000;
    font-size: 9.5pt;
  }
  header {
    margin-bottom: 4mm;
    border-bottom: 0.4mm solid #000;
    padding-bottom: 2mm;
  }
  .shop {
    font-size: 9pt;
    color: #444;
  }
  h1 {
    font-size: 15pt;
    margin: 1mm 0;
  }
  .sub,
  .gen {
    font-size: 8.5pt;
    color: #333;
  }
  table {
    width: 100%;
    border-collapse: collapse;
  }
  th,
  td {
    text-align: left;
    padding: 1.2mm 1.6mm;
    border-bottom: 0.2mm solid #bbb;
    vertical-align: top;
  }
  th {
    font-size: 8pt;
    text-transform: uppercase;
    border-bottom: 0.4mm solid #000;
  }
  thead {
    display: table-header-group;
  }
  tr {
    break-inside: avoid;
  }
  .num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
</style>
