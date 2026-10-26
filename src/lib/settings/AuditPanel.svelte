<script lang="ts">
  import { api, type AuditRow, type VerifyReport } from '../api';
  import { app } from '../app.svelte';
  import Icon from '../components/Icon.svelte';
  import { dateTime } from '../format';
  import { auditDiff, auditLabel } from '../labels';

  let report = $state<VerifyReport | null>(null);
  let verifying = $state(false);
  let rows = $state<AuditRow[]>([]);
  let more = $state(true);

  $effect(() => {
    void app.dataVersion;
    load(true);
  });

  async function load(reset = false) {
    try {
      const page = await api.auditLog({ limit: 50, beforeId: reset ? undefined : rows.at(-1)?.id });
      rows = reset ? page : [...rows, ...page];
      more = page.length === 50;
    } catch (e) {
      app.error(e);
    }
  }

  async function verify() {
    verifying = true;
    try {
      report = (await app.withTech(() => api.verifyAudit())) ?? null;
    } catch (e) {
      app.error(e);
    } finally {
      verifying = false;
    }
  }
</script>

<section class="card card-pad stack">
  <div class="row wrap">
    <div class="grow">
      <h2>Audit log</h2>
      <p class="muted">
        Every action is recorded with who did it. Entries are hash-chained: editing, inserting or deleting any entry breaks
        the chain from that point on.
      </p>
    </div>
    <button class="btn btn-primary" onclick={verify} disabled={verifying}>
      <Icon name="shield" /> Verify log integrity
    </button>
  </div>

  {#if report}
    {#if report.ok}
      <div class="result ok" role="status">
        <Icon name="check" />
        <div>
          <strong>Intact.</strong> All {report.entries} entries verified.
          <div class="faint mono small">Chain head {report.headHash.slice(0, 16)}…</div>
        </div>
      </div>
    {:else}
      <div class="result bad" role="alert">
        <Icon name="alert" />
        <div>
          <strong>Broken at entry #{report.firstBroken?.id}:</strong>
          {report.firstBroken?.reason}. The first {report.entries} entries are intact. Restore a backup or keep this file
          for investigation.
        </div>
      </div>
    {/if}
  {/if}

  <div class="scroll">
    <table class="table">
      <thead>
        <tr><th>#</th><th>When</th><th>Who</th><th>Action</th><th>Details</th></tr>
      </thead>
      <tbody>
        {#each rows as r (r.id)}
          <tr>
            <td class="faint">{r.id}</td>
            <td class="nowrap">{dateTime(r.ts)}</td>
            <td>
              {r.actorName}
              {#if r.actorKind !== 'technician'}<span class="faint small">({r.actorKind})</span>{/if}
            </td>
            <td class="nowrap">{auditLabel(r.action)}</td>
            <td class="details">
              <span class="faint">{r.entityKind}{r.entityId != null ? ` #${r.entityId}` : ''}</span>
              {auditDiff(r.beforeJson, r.afterJson)}
            </td>
          </tr>
        {:else}
          <tr><td colspan="5" class="empty">Nothing logged yet</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
  {#if more && rows.length}
    <button class="btn" onclick={() => load()}>Load older entries</button>
  {/if}
</section>

<style>
  .result {
    display: flex;
    gap: 0.75rem;
    padding: 0.85rem 1rem;
    border-radius: var(--radius);
  }
  .result.ok {
    background: var(--success-soft);
    color: var(--success);
  }
  .result.bad {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .result div {
    color: var(--text);
  }
  .small {
    font-size: 0.8rem;
  }
  .scroll {
    max-height: 480px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .details {
    font-size: 0.88rem;
    word-break: break-word;
  }
</style>
