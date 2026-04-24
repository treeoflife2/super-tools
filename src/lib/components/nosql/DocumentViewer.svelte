<script lang="ts">
  import { showToast } from '$lib/components/shared/toast';
  import { friendlyError } from '$lib/utils/errors';
  import { nosqlFindDocuments, nosqlInsertDocument, nosqlUpdateDocument, nosqlDeleteDocument, nosqlCountDocuments } from '$lib/commands/nosql_client';
  import { parseCsv, parseJsonDocs } from '$lib/utils/import-parser';
  import QueryEditor from './QueryEditor.svelte';
  import { highlightJSON } from '$lib/utils/json-highlight';
  import { writeText } from '@tauri-apps/plugin-clipboard-manager';

  interface Props {
    connectionId: string;
    database: string;
    collection: string;
    initialFilter?: string;
  }

  let { connectionId, database, collection, initialFilter }: Props = $props();

  type ViewTab = 'documents' | 'query' | 'aggregation';
  let activeTab = $state<ViewTab>('documents');

  // Document listing
  let documents = $state<any[]>([]);
  let totalCount = $state(0);
  let durationMs = $state(0);
  let loading = $state(false);
  let limit = $state(25);
  let skip = $state(0);
  // Committed filter/sort (used for actual queries)
  let filterQuery = $state('{}');
  let sortQuery = $state('{}');
  // Input values (local state, committed on Enter/Find)
  let filterInput = $state('{}');
  let sortInput = $state('{}');
  let showOptions = $state(false);
  let expandedDocs = $state<Set<number>>(new Set());
  let editingIdx = $state<number | null>(null);
  let editingValue = $state('');

  // Search within documents
  let showSearch = $state(false);
  let searchQuery = $state('');
  let searchInputRef = $state<HTMLInputElement | null>(null);
  let currentMatchIndex = $state(0);
  let docListRef = $state<HTMLDivElement | null>(null);

  // Query / Aggregation
  let queryValue = $state('{}');
  let aggPipeline = $state('[]');
  let queryResult = $state<any[] | null>(null);
  let queryDuration = $state(0);

  // Add data dropdown
  let showAddMenu = $state(false);
  let fileInput = $state<HTMLInputElement | null>(null);

  // Close add menu on click outside
  function handleWindowClick() { if (showAddMenu) showAddMenu = false; }

  // Generate random ObjectId (24 hex chars)
  function generateObjectId(): string {
    const hex = '0123456789abcdef';
    let id = '';
    for (let i = 0; i < 24; i++) id += hex[Math.floor(Math.random() * 16)];
    return id;
  }

  function makeInsertTemplate(): string {
    return `{\n  "_id": {\n    "$oid": "${generateObjectId()}"\n  }\n}`;
  }

  // Count documents in editor (single object or array)
  function countInsertDocs(text: string): number {
    try {
      const parsed = JSON.parse(text);
      if (Array.isArray(parsed)) return parsed.length;
      return 1;
    } catch { return 0; }
  }

  // Insert/Update/Delete modals
  let showInsert = $state(false);
  let insertDoc = $state(makeInsertTemplate());
  let insertValid = $state(true);
  let insertError = $state('');
  let insertErrorLine = $state(-1);
  let insertDocCount = $derived(insertValid ? countInsertDocs(insertDoc) : 0);
  let showUpdateModal = $state(false);
  let updateFilter = $state('');
  let updateDoc = $state('');
  let updateFilterValid = $state(true);
  let updateDocValid = $state(true);
  let updateFilterError = $state('');
  let updateDocError = $state('');
  let updateFilterErrorLine = $state(-1);
  let updateDocErrorLine = $state(-1);
  let showDeleteModal = $state(false);
  let deleteFilter = $state('');
  let deleteFilterValid = $state(true);
  let deleteFilterError = $state('');
  let deleteFilterErrorLine = $state(-1);

  // Track previous collection to detect actual changes
  let prevKey = $state('');
  // Track the last AI filter we applied so we don't re-apply it after user reset
  let appliedAiFilter = $state('');

  // Reload when collection changes
  $effect(() => {
    const key = `${connectionId}:${database}:${collection}`;
    if (connectionId && database && collection && key !== prevKey) {
      prevKey = key;
      skip = 0;
      limit = 25;
      const filter = initialFilter && initialFilter !== '{}' ? initialFilter : '{}';
      filterQuery = filter;
      filterInput = filter;
      appliedAiFilter = initialFilter ?? '';
      sortQuery = '{}';
      sortInput = '{}';
      expandedDocs = new Set();
      queueMicrotask(() => loadDocuments());
    }
  });

  // Apply AI-injected filter changes — only when a NEW filter arrives from AI
  $effect(() => {
    if (initialFilter && initialFilter !== '{}' && initialFilter !== appliedAiFilter) {
      appliedAiFilter = initialFilter;
      filterQuery = initialFilter;
      filterInput = initialFilter;
      skip = 0;
      queueMicrotask(() => loadDocuments());
    }
  });

  function resetFilters() {
    filterQuery = '{}';
    filterInput = '{}';
    sortQuery = '{}';
    sortInput = '{}';
    limit = 25;
    skip = 0;
    queueMicrotask(() => loadDocuments());
  }

  async function loadDocuments() {
    loading = true;
    try {
      const result = await nosqlFindDocuments(connectionId, database, collection, filterQuery, sortQuery, limit, skip);
      documents = result.documents;
      durationMs = result.durationMs;
      totalCount = result.totalCount ?? 0;
    } catch (e: any) {
      showToast(friendlyError(e), 'error');
      documents = [];
    } finally {
      loading = false;
    }
  }

  function executeFilter() {
    filterQuery = filterInput.trim() || '{}';
    sortQuery = sortInput.trim() || '{}';
    skip = 0;
    expandedDocs = new Set();
    loadDocuments();
  }

  function clearFilter() {
    filterInput = '{}';
    sortInput = '{}';
    filterQuery = '{}';
    sortQuery = '{}';
    appliedAiFilter = '';
    limit = 25;
    skip = 0;
    expandedDocs = new Set();
    loadDocuments();
  }

  const hasActiveFilter = $derived(filterInput !== '{}' || sortInput !== '{}');

  // Search helpers
  function escapeRegex(s: string) { return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'); }

  const matchCount = $derived.by(() => {
    if (!searchQuery || documents.length === 0) return 0;
    const regex = new RegExp(escapeRegex(searchQuery), 'gi');
    let count = 0;
    for (let i = 0; i < documents.length; i++) {
      if (!expandedDocs.has(i)) continue;
      const text = JSON.stringify(documents[i], null, 2);
      const matches = text.match(regex);
      if (matches) count += matches.length;
    }
    return count;
  });

  function highlightSearchInDoc(json: string): string {
    if (!searchQuery) return highlightJSON(json);
    // First apply JSON syntax highlighting, then overlay search matches
    const highlighted = highlightJSON(json);
    const regex = new RegExp(`(${escapeRegex(searchQuery)})`, 'gi');
    let matchIdx = -1;
    return highlighted.replace(/>([^<]*)</g, (full, text) => {
      return '>' + text.replace(regex, (_m: string) => {
        matchIdx++;
        const cls = matchIdx === currentMatchIndex ? 'dv-search-match dv-active-match' : 'dv-search-match';
        return `<span class="${cls}">${_m}</span>`;
      }) + '<';
    });
  }

  function handleDocKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'f') {
      e.preventDefault();
      showSearch = true;
      setTimeout(() => searchInputRef?.focus(), 0);
    }
  }

  function handleSearchKeydown(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      closeSearch();
    } else if (e.key === 'Enter' && matchCount > 0) {
      if (e.shiftKey) {
        currentMatchIndex = (currentMatchIndex - 1 + matchCount) % matchCount;
      } else {
        currentMatchIndex = (currentMatchIndex + 1) % matchCount;
      }
      scrollToSearchMatch();
    }
  }

  function scrollToSearchMatch() {
    setTimeout(() => {
      const active = docListRef?.querySelector('.dv-active-match');
      if (active) active.scrollIntoView({ block: 'center', behavior: 'smooth' });
    }, 10);
  }

  function closeSearch() {
    showSearch = false;
    searchQuery = '';
    currentMatchIndex = 0;
  }

  $effect(() => {
    if (searchQuery) currentMatchIndex = 0;
  });

  function nextPage() {
    skip += limit;
    expandedDocs = new Set();
    loadDocuments();
  }

  function prevPage() {
    skip = Math.max(0, skip - limit);
    expandedDocs = new Set();
    loadDocuments();
  }

  function toggleExpand(i: number) {
    const next = new Set(expandedDocs);
    if (next.has(i)) next.delete(i); else next.add(i);
    expandedDocs = next;
  }

  async function copyDocument(doc: any) {
    try {
      await writeText(JSON.stringify(doc, null, 2));
      showToast('Document copied', 'success');
    } catch {
      showToast('Failed to copy', 'error');
    }
  }

  function editDocument(doc: any) {
    const id = doc._id;
    updateFilter = JSON.stringify({ _id: id }, null, 2);
    // Exclude _id from $set — MongoDB throws if you try to update _id
    const { _id, ...rest } = doc;
    updateDoc = JSON.stringify({ $set: rest }, null, 2);
    showUpdateModal = true;
  }

  function validateJson(value: string): { valid: boolean; error: string; errorLine: number } {
    try {
      JSON.parse(value);
      return { valid: true, error: '', errorLine: -1 };
    } catch (err: any) {
      const msg = err.message ?? 'Invalid JSON';
      let line = -1;
      const m = msg.match(/position (\d+)/);
      if (m) {
        line = value.slice(0, parseInt(m[1])).split('\n').length;
      }
      return { valid: false, error: msg, errorLine: line };
    }
  }

  function handleInsertInput(e: Event) {
    insertDoc = (e.target as HTMLTextAreaElement).value;
    const r = validateJson(insertDoc);
    insertValid = r.valid;
    insertError = r.error;
    insertErrorLine = r.errorLine;
  }

  function handleUpdateFilterInput(e: Event) {
    updateFilter = (e.target as HTMLTextAreaElement).value;
    const r = validateJson(updateFilter);
    updateFilterValid = r.valid;
    updateFilterError = r.error;
    updateFilterErrorLine = r.errorLine;
  }

  function handleUpdateDocInput(e: Event) {
    updateDoc = (e.target as HTMLTextAreaElement).value;
    const r = validateJson(updateDoc);
    updateDocValid = r.valid;
    updateDocError = r.error;
    updateDocErrorLine = r.errorLine;
  }

  function handleDeleteFilterInput(e: Event) {
    deleteFilter = (e.target as HTMLTextAreaElement).value;
    const r = validateJson(deleteFilter);
    deleteFilterValid = r.valid;
    deleteFilterError = r.error;
    deleteFilterErrorLine = r.errorLine;
  }

  function handleEditInput(e: Event) {
    editingValue = (e.target as HTMLTextAreaElement).value;
    validateEditJson();
  }

  let editJsonValid = $state(true);
  let editErrorLine = $state(-1);
  let editErrorMsg = $state('');

  // Compute textarea height from line count (19px per line + 16px padding)
  function taHeight(value: string): string {
    if (!value) return 'auto';
    return (value.split('\n').length * 19 + 16) + 'px';
  }
  const editTaHeight = $derived(taHeight(editingValue));

  function startInlineEdit(idx: number, doc: any) {
    editingIdx = idx;
    editingValue = JSON.stringify(doc, null, 2);
    validateEditJson();
  }

  function cancelInlineEdit() {
    editingIdx = null;
    editingValue = '';
    editJsonValid = true;
    editErrorLine = -1;
    editErrorMsg = '';
  }

  function validateEditJson() {
    try {
      JSON.parse(editingValue);
      editJsonValid = true;
      editErrorLine = -1;
      editErrorMsg = '';
    } catch (err: any) {
      editJsonValid = false;
      editErrorMsg = err.message ?? 'Invalid JSON';
      // Try to extract line number from error message
      const lineMatch = editErrorMsg.match(/position (\d+)/);
      if (lineMatch) {
        const pos = parseInt(lineMatch[1]);
        const before = editingValue.slice(0, pos);
        editErrorLine = before.split('\n').length;
      } else {
        editErrorLine = -1;
      }
    }
  }

  async function saveInlineEdit(originalDoc: any) {
    if (editingIdx === null || !editJsonValid) return;
    try {
      const updatedDoc = JSON.parse(editingValue);
      const id = originalDoc._id;
      const filter = JSON.stringify({ _id: id });
      // Exclude _id from $set
      const { _id, ...rest } = updatedDoc;
      const update = JSON.stringify({ $set: rest });
      await nosqlUpdateDocument(connectionId, database, collection, filter, update);
      showToast('Document updated', 'success');
      cancelInlineEdit();
      loadDocuments();
    } catch (e: any) {
      showToast(friendlyError(e), 'error');
    }
  }

  // Generate highlighted lines with line numbers for edit view
  function getEditLines(value: string): { num: number; html: string; hasError: boolean }[] {
    const rawLines = value.split('\n');
    return rawLines.map((line, i) => ({
      num: i + 1,
      html: highlightJSONLine(line),
      hasError: editErrorLine === i + 1,
    }));
  }

  function highlightJSONLine(line: string): string {
    const escaped = line.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
    return escaped.replace(
      /("(?:[^"\\]|\\.)*")\s*(:)|("(?:[^"\\]|\\.)*")|((?:-?\d+\.?\d*(?:[eE][+-]?\d+)?))|(\btrue\b|\bfalse\b|\bnull\b)|([{}[\]:,])/g,
      (_m, key, colon, str, num, boo, pu) => {
        if (key !== undefined && colon !== undefined) return `<span class="key">${key}</span><span class="pu">${colon}</span>`;
        if (str !== undefined) return `<span class="str">${str}</span>`;
        if (num !== undefined) return `<span class="num">${num}</span>`;
        if (boo !== undefined) return `<span class="boo">${boo}</span>`;
        if (pu !== undefined) return `<span class="pu">${pu}</span>`;
        return _m;
      }
    );
  }

  const editLines = $derived(editingValue ? getEditLines(editingValue) : []);

  function deleteDocument(doc: any) {
    const id = doc._id;
    deleteFilter = JSON.stringify({ _id: id }, null, 2);
    showDeleteModal = true;
  }

  async function handleInsert() {
    try {
      const parsed = JSON.parse(insertDoc);
      const docs = Array.isArray(parsed) ? parsed : [parsed];
      for (const doc of docs) {
        await nosqlInsertDocument(connectionId, database, collection, JSON.stringify(doc));
      }
      showToast(`${docs.length} document${docs.length > 1 ? 's' : ''} inserted`, 'success');
      showInsert = false;
      insertDoc = makeInsertTemplate();
      loadDocuments();
    } catch (e: any) {
      showToast(friendlyError(e), 'error');
    }
  }

  function handleInsertKeydown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 'Enter' && insertValid) {
      e.preventDefault();
      handleInsert();
    }
  }

  function formatInsertDoc() {
    try {
      const parsed = JSON.parse(insertDoc);
      insertDoc = JSON.stringify(parsed, null, 2);
      const r = validateJson(insertDoc);
      insertValid = r.valid;
      insertError = r.error;
      insertErrorLine = r.errorLine;
    } catch { /* already invalid, do nothing */ }
  }

  async function handleFileImport(e: Event) {
    const input = e.target as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    input.value = '';

    try {
      const text = await file.text();
      let docs: any[];

      if (file.name.endsWith('.csv')) {
        docs = parseCsv(text);
      } else {
        docs = parseJsonDocs(text);
      }

      if (docs.length === 0) { showToast('No documents found in file', 'error'); return; }

      let inserted = 0;
      for (const doc of docs) {
        await nosqlInsertDocument(connectionId, database, collection, JSON.stringify(doc));
        inserted++;
      }
      showToast(`Imported ${inserted} document${inserted > 1 ? 's' : ''}`, 'success');
      loadDocuments();
    } catch (e: any) {
      showToast(friendlyError(e), 'error');
    }
  }

  async function handleUpdate() {
    try {
      const count = await nosqlUpdateDocument(connectionId, database, collection, updateFilter, updateDoc);
      showToast(`${count} document(s) updated`, 'success');
      showUpdateModal = false;
      loadDocuments();
    } catch (e: any) {
      showToast(friendlyError(e), 'error');
    }
  }

  async function handleDelete() {
    try {
      const count = await nosqlDeleteDocument(connectionId, database, collection, deleteFilter);
      showToast(`${count} document(s) deleted`, 'success');
      showDeleteModal = false;
      loadDocuments();
    } catch (e: any) {
      showToast(friendlyError(e), 'error');
    }
  }

  async function handleQueryExecute(val: string) {
    queryValue = val;
    loading = true;
    try {
      const result = await nosqlFindDocuments(connectionId, database, collection, val, sortQuery, 100, 0);
      queryResult = result.documents;
      queryDuration = result.durationMs;
    } catch (e: any) {
      showToast(friendlyError(e), 'error');
    } finally {
      loading = false;
    }
  }

  async function handleAggExecute(val: string) {
    aggPipeline = val;
    loading = true;
    try {
      const { nosqlAggregate } = await import('$lib/commands/nosql_client');
      const result = await nosqlAggregate(connectionId, database, collection, val);
      queryResult = result.documents;
      queryDuration = result.durationMs;
    } catch (e: any) {
      showToast(friendlyError(e), 'error');
    } finally {
      loading = false;
    }
  }

  function displayId(id: any): string {
    if (!id) return '';
    if (typeof id === 'string') return id;
    if (typeof id === 'object' && id.$oid) return id.$oid;
    return JSON.stringify(id);
  }

  function formatDoc(doc: any): string {
    try {
      return JSON.stringify(doc, null, 2);
    } catch {
      return String(doc);
    }
  }

  function docPreview(doc: any): string {
    const keys = Object.keys(doc).filter(k => k !== '_id').slice(0, 4);
    const parts = keys.map(k => {
      const v = doc[k];
      const val = typeof v === 'string' ? `"${v.length > 30 ? v.slice(0, 30) + '...' : v}"` :
                  v === null ? 'null' :
                  typeof v === 'object' ? (Array.isArray(v) ? `[${v.length}]` : '{...}') :
                  String(v);
      return `${k}: ${val}`;
    });
    return parts.join('  ·  ') + (Object.keys(doc).length > keys.length + 1 ? '  ...' : '');
  }

  const currentPage = $derived(Math.floor(skip / limit) + 1);
  const totalPages = $derived(Math.max(1, Math.ceil(totalCount / limit)));
</script>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<svelte:window onclick={handleWindowClick} />
<div class="dv-wrap">
  <!-- Mode tabs + actions -->
  <div class="dv-tabs">
    <button class="dv-tab" class:on={activeTab === 'documents'} onclick={() => activeTab = 'documents'}>
      <svg viewBox="0 0 24 24" width="12" height="12"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
      Documents
    </button>
    <button class="dv-tab" class:on={activeTab === 'query'} onclick={() => activeTab = 'query'}>
      <svg viewBox="0 0 24 24" width="12" height="12"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
      Query
    </button>
    <button class="dv-tab" class:on={activeTab === 'aggregation'} onclick={() => activeTab = 'aggregation'}>
      <svg viewBox="0 0 24 24" width="12" height="12"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/></svg>
      Aggregation
    </button>
    <div class="dv-tab-spacer"></div>
    {#if activeTab === 'documents'}
      <span class="dv-meta">{totalCount.toLocaleString()} docs</span>
      <span class="dv-meta dv-meta-sep">·</span>
      <span class="dv-meta">{durationMs}ms</span>
      <div class="dv-add-wrap">
        <button class="dv-toolbar-btn" onclick={(e) => { e.stopPropagation(); showAddMenu = !showAddMenu; }} title="Add Data">
          <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="M12 5v14M5 12h14"/></svg>
          Add Data
          <svg viewBox="0 0 24 24" width="9" height="9" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d="M6 9l6 6 6-6"/></svg>
        </button>
        {#if showAddMenu}
          <!-- svelte-ignore a11y_click_events_have_key_events -->
          <!-- svelte-ignore a11y_no_static_element_interactions -->
          <div class="dv-add-menu" onclick={() => showAddMenu = false}>
            <button class="dv-add-menu-item" onclick={() => { showAddMenu = false; insertDoc = makeInsertTemplate(); insertValid = true; insertError = ''; insertErrorLine = -1; showInsert = true; }}>
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><path d="M12 11v6M9 14h6"/></svg>
              Insert Document
            </button>
            <button class="dv-add-menu-item" onclick={() => { showAddMenu = false; fileInput?.click(); }}>
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="17 8 12 3 7 8"/><line x1="12" y1="3" x2="12" y2="15"/></svg>
              Import JSON or CSV File
            </button>
          </div>
        {/if}
        <input bind:this={fileInput} type="file" accept=".json,.csv" style="display:none" onchange={handleFileImport} />
      </div>
      <button class="dv-toolbar-btn" onclick={() => loadDocuments()} title="Refresh">
        <svg viewBox="0 0 24 24" width="13" height="13" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/></svg>
      </button>
    {/if}
  </div>

  {#if activeTab === 'documents'}
    <!-- Filter bar -->
    <div class="dv-filter-bar">
      <textarea
        class="dv-filter-input"
        bind:value={filterInput}
        placeholder={'{"field": "value"}'}
        spellcheck="false"
        rows="1"
        oninput={(e) => {
          const t = e.currentTarget;
          t.style.height = 'auto';
          t.style.height = Math.min(t.scrollHeight, 200) + 'px';
        }}
        onkeydown={(e) => {
          if (e.key === 'Enter' && (e.metaKey || e.ctrlKey)) {
            // Cmd+Enter always executes the filter
            e.preventDefault();
            executeFilter();
          }
        }}
      ></textarea>
      <div class="dv-bar-actions">
        <button class="dv-bar-btn dv-find-btn" onclick={executeFilter}>Find</button>
        <button class="dv-bar-btn dv-reset-btn" onclick={clearFilter}>Reset</button>
        <button class="dv-bar-btn dv-options-btn" class:active={showOptions} onclick={() => showOptions = !showOptions}>
          Options
          <svg viewBox="0 0 24 24" width="9" height="9" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><path d={showOptions ? 'M18 15l-6-6-6 6' : 'M6 9l6 6 6-6'}/></svg>
        </button>
        <select class="dv-limit-sel" bind:value={limit} onchange={() => { skip = 0; loadDocuments(); }} title="Per page">
          <option value={10}>10</option>
          <option value={25}>25</option>
          <option value={50}>50</option>
          <option value={100}>100</option>
        </select>
        <button class="dv-page-btn" onclick={prevPage} disabled={skip === 0}>
          <svg viewBox="0 0 24 24" width="10" height="10"><polyline points="15 18 9 12 15 6"/></svg>
        </button>
        <span class="dv-page-info">{currentPage} / {totalPages}</span>
        <button class="dv-page-btn" onclick={nextPage} disabled={skip + limit >= totalCount}>
          <svg viewBox="0 0 24 24" width="10" height="10"><polyline points="9 18 15 12 9 6"/></svg>
        </button>
      </div>
    </div>

    {#if showOptions}
      <div class="dv-options-row">
        <span class="dv-option-label">Sort</span>
        <input
          class="dv-option-input"
          type="text"
          bind:value={sortInput}
          placeholder={'{"_id": -1}'}
          spellcheck="false"
          onkeydown={(e) => { if (e.key === 'Enter') executeFilter(); }}
        />
      </div>
    {/if}

    <!-- Search bar -->
    {#if showSearch}
      <div class="dv-search-bar">
        <svg viewBox="0 0 24 24" width="13" height="13"><circle cx="11" cy="11" r="8" stroke="currentColor" fill="none" stroke-width="2"/><path d="M21 21l-4.35-4.35" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"/></svg>
        <input
          bind:this={searchInputRef}
          class="dv-search-input"
          type="text"
          placeholder="Search in documents..."
          bind:value={searchQuery}
          onkeydown={handleSearchKeydown}
        />
        {#if searchQuery}
          <span class="dv-search-count">{matchCount > 0 ? `${currentMatchIndex + 1} of ${matchCount}` : 'No matches'}</span>
        {/if}
        <button class="dv-search-nav" onclick={() => { currentMatchIndex = (currentMatchIndex - 1 + matchCount) % matchCount; scrollToSearchMatch(); }} disabled={matchCount === 0}>
          <svg viewBox="0 0 24 24" width="11" height="11"><path d="M18 15l-6-6-6 6" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"/></svg>
        </button>
        <button class="dv-search-nav" onclick={() => { currentMatchIndex = (currentMatchIndex + 1) % matchCount; scrollToSearchMatch(); }} disabled={matchCount === 0}>
          <svg viewBox="0 0 24 24" width="11" height="11"><path d="M6 9l6 6 6-6" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"/></svg>
        </button>
        <button class="dv-search-nav" onclick={closeSearch}>
          <svg viewBox="0 0 24 24" width="11" height="11"><path d="M18 6L6 18M6 6l12 12" stroke="currentColor" fill="none" stroke-width="2" stroke-linecap="round"/></svg>
        </button>
      </div>
    {/if}

    <!-- Document list -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="dv-list" bind:this={docListRef} onkeydown={handleDocKeydown} tabindex="-1">
      {#if loading}
        <div class="dv-empty-state">
          <svg viewBox="0 0 24 24" width="32" height="32"><ellipse cx="12" cy="5" rx="9" ry="3"/><path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3"/><path d="M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/></svg>
          <span>Loading<span class="dv-dots"></span></span>
        </div>
      {:else if documents.length === 0}
        <div class="dv-empty-state">
          <svg viewBox="0 0 24 24" width="32" height="32"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z"/><polyline points="14 2 14 8 20 8"/></svg>
          <span>No documents found</span>
        </div>
      {:else}
        {#each documents as doc, i (i)}
          {@const isExpanded = expandedDocs.has(i)}
          <div class="dv-doc" class:expanded={isExpanded}>
            <!-- svelte-ignore a11y_click_events_have_key_events -->
            <!-- svelte-ignore a11y_no_static_element_interactions -->
            <div class="dv-doc-header" onclick={() => toggleExpand(i)}>
              <span class="dv-doc-chevron" class:open={isExpanded}>
                <svg viewBox="0 0 24 24" width="10" height="10"><polyline points="9 18 15 12 9 6"/></svg>
              </span>
              <span class="dv-doc-idx">#{skip + i + 1}</span>
              <span class="dv-doc-id">{displayId(doc._id)}</span>
              <div class="dv-doc-actions">
                <button class="dv-doc-btn" title="Copy" onclick={(e) => { e.stopPropagation(); copyDocument(doc); }}>
                  <svg viewBox="0 0 24 24" width="12" height="12"><rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/></svg>
                </button>
                <button class="dv-doc-btn" title="Edit" onclick={(e) => { e.stopPropagation(); editDocument(doc); }}>
                  <svg viewBox="0 0 24 24" width="12" height="12"><path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
                </button>
                <button class="dv-doc-btn danger" title="Delete" onclick={(e) => { e.stopPropagation(); deleteDocument(doc); }}>
                  <svg viewBox="0 0 24 24" width="12" height="12"><polyline points="3 6 5 6 21 6"/><path d="M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2"/></svg>
                </button>
              </div>
            </div>
            {#if isExpanded}
              {#if editingIdx === i}
                <div class="dv-doc-edit-wrap">
                  <div class="dv-edit-editor">
                    <div class="dv-edit-gutter">
                      {#each editingValue.split('\n') as _, ln}
                        <span class="dv-edit-ln" class:dv-ln-err={editErrorLine === ln + 1}>{ln + 1}</span>
                      {/each}
                    </div>
                    <textarea
                      class="dv-edit-ta"
                      style="height:{editTaHeight}"
                      value={editingValue}
                      oninput={handleEditInput}
                      spellcheck="false"
                    ></textarea>
                  </div>
                  {#if !editJsonValid}
                    <div class="dv-edit-error">
                      <svg viewBox="0 0 24 24" width="12" height="12"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
                      {editErrorMsg}
                    </div>
                  {/if}
                  <div class="dv-doc-edit-actions">
                    <button class="dv-edit-cancel" onclick={cancelInlineEdit}>Cancel</button>
                    <button class="dv-edit-save" disabled={!editJsonValid} onclick={() => saveInlineEdit(doc)}>Update</button>
                  </div>
                </div>
              {:else}
                <!-- svelte-ignore a11y_no_static_element_interactions -->
                <div class="dv-doc-body" ondblclick={() => startInlineEdit(i, doc)}>
                  {@html searchQuery ? highlightSearchInDoc(formatDoc(doc)) : highlightJSON(formatDoc(doc))}
                </div>
              {/if}
            {:else}
              <div class="dv-doc-preview">{docPreview(doc)}</div>
            {/if}
          </div>
        {/each}
      {/if}
    </div>


    <!-- Insert Document modal (Compass-style) -->
    {#if showInsert}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="dv-modal-overlay" onclick={(e) => { if (e.target === e.currentTarget) showInsert = false; }}>
        <div class="dv-modal dv-insert-modal">
          <div class="dv-insert-header">
            <div>
              <div class="dv-insert-title">Insert Document</div>
              <div class="dv-insert-subtitle">To collection <strong>{database}</strong>.<strong>{collection}</strong></div>
            </div>
            <button class="dv-modal-close" onclick={() => showInsert = false}>
              <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><path d="M18 6L6 18M6 6l12 12"/></svg>
            </button>
          </div>
          <div class="dv-insert-toolbar">
            <button class="dv-insert-tool-btn" onclick={formatInsertDoc} title="Format JSON">
              <svg viewBox="0 0 24 24" width="14" height="14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
              Format
            </button>
            {#if insertDocCount > 0}
              <span class="dv-insert-doc-count">{insertDocCount} document{insertDocCount > 1 ? 's' : ''}</span>
            {/if}
          </div>
          <div class="dv-insert-editor-area">
            <div class="dv-insert-editor-container">
              <div class="dv-edit-editor dv-insert-editor">
                <div class="dv-edit-gutter">
                  {#each insertDoc.split('\n') as _, ln}
                    <span class="dv-edit-ln" class:dv-ln-err={insertErrorLine === ln + 1}>{ln + 1}</span>
                  {/each}
                </div>
                <textarea
                  class="dv-edit-ta"
                  style="min-height:300px"
                  value={insertDoc}
                  oninput={handleInsertInput}
                  onkeydown={handleInsertKeydown}
                  spellcheck="false"
                  placeholder="Paste one or more documents here"
                ></textarea>
              </div>
              <div class="dv-insert-highlight" aria-hidden="true">
                {@html highlightJSON(insertDoc)}
              </div>
            </div>
            {#if !insertValid}
              <div class="dv-edit-error">
                <svg viewBox="0 0 24 24" width="12" height="12"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
                {insertError}
              </div>
            {/if}
          </div>
          <div class="dv-insert-footer">
            <span class="dv-insert-hint">
              <kbd>&#8984;Enter</kbd> to insert
            </span>
            <button class="dv-modal-cancel" onclick={() => showInsert = false}>Cancel</button>
            <button class="dv-modal-primary" disabled={!insertValid} onclick={handleInsert}>
              Insert{insertDocCount > 1 ? ` ${insertDocCount} Docs` : ''}
            </button>
          </div>
        </div>
      </div>
    {/if}

    <!-- Update modal -->
    {#if showUpdateModal}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="dv-modal-overlay" onclick={(e) => { if (e.target === e.currentTarget) showUpdateModal = false; }}>
        <div class="dv-modal dv-modal-lg">
          <div class="dv-modal-hdr">
            <span>Update Document</span>
            <button class="dv-modal-close" onclick={() => showUpdateModal = false}>&times;</button>
          </div>
          <div class="dv-modal-editor-wrap">
            <span class="dv-modal-label">Filter</span>
            <div class="dv-edit-editor dv-modal-editor-sm">
              <div class="dv-edit-gutter">
                {#each updateFilter.split('\n') as _, ln}
                  <span class="dv-edit-ln" class:dv-ln-err={updateFilterErrorLine === ln + 1}>{ln + 1}</span>
                {/each}
              </div>
              <textarea class="dv-edit-ta" style="height:{taHeight(updateFilter)}" value={updateFilter} oninput={handleUpdateFilterInput} spellcheck="false"></textarea>
            </div>
            {#if !updateFilterValid}
              <div class="dv-edit-error">
                <svg viewBox="0 0 24 24" width="12" height="12"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
                {updateFilterError}
              </div>
            {/if}
          </div>
          <div class="dv-modal-editor-wrap">
            <span class="dv-modal-label">Update</span>
            <div class="dv-edit-editor dv-modal-editor">
              <div class="dv-edit-gutter">
                {#each updateDoc.split('\n') as _, ln}
                  <span class="dv-edit-ln" class:dv-ln-err={updateDocErrorLine === ln + 1}>{ln + 1}</span>
                {/each}
              </div>
              <textarea class="dv-edit-ta" style="height:{taHeight(updateDoc)}" value={updateDoc} oninput={handleUpdateDocInput} spellcheck="false"></textarea>
            </div>
            {#if !updateDocValid}
              <div class="dv-edit-error">
                <svg viewBox="0 0 24 24" width="12" height="12"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
                {updateDocError}
              </div>
            {/if}
          </div>
          <div class="dv-modal-actions">
            <button class="dv-modal-cancel" onclick={() => showUpdateModal = false}>Cancel</button>
            <button class="dv-modal-primary" disabled={!updateFilterValid || !updateDocValid} onclick={handleUpdate}>Update</button>
          </div>
        </div>
      </div>
    {/if}

    <!-- Delete modal -->
    {#if showDeleteModal}
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="dv-modal-overlay" onclick={(e) => { if (e.target === e.currentTarget) showDeleteModal = false; }}>
        <div class="dv-modal dv-modal-lg">
          <div class="dv-modal-hdr">
            <span>Delete Document</span>
            <button class="dv-modal-close" onclick={() => showDeleteModal = false}>&times;</button>
          </div>
          <div class="dv-modal-editor-wrap">
            <span class="dv-modal-label">Filter</span>
            <div class="dv-edit-editor dv-modal-editor-sm">
              <div class="dv-edit-gutter">
                {#each deleteFilter.split('\n') as _, ln}
                  <span class="dv-edit-ln" class:dv-ln-err={deleteFilterErrorLine === ln + 1}>{ln + 1}</span>
                {/each}
              </div>
              <textarea class="dv-edit-ta" style="height:{taHeight(deleteFilter)}" value={deleteFilter} oninput={handleDeleteFilterInput} spellcheck="false"></textarea>
            </div>
            {#if !deleteFilterValid}
              <div class="dv-edit-error">
                <svg viewBox="0 0 24 24" width="12" height="12"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
                {deleteFilterError}
              </div>
            {/if}
          </div>
          <div class="dv-modal-actions">
            <button class="dv-modal-cancel" onclick={() => showDeleteModal = false}>Cancel</button>
            <button class="dv-modal-primary danger" disabled={!deleteFilterValid} onclick={handleDelete}>Delete</button>
          </div>
        </div>
      </div>
    {/if}
  {:else if activeTab === 'query'}
    <div class="dv-query-area">
      <QueryEditor bind:value={queryValue} label="Find Query" placeholder={'{"field": "value"}'} onexecute={handleQueryExecute} />
    </div>
    {#if queryResult}
      <div class="dv-query-result">
        <div class="dv-query-meta">
          <span>{queryResult.length} results</span>
          <span class="dv-meta-sep">·</span>
          <span>{queryDuration}ms</span>
        </div>
        <div class="dv-query-body">{@html highlightJSON(JSON.stringify(queryResult, null, 2))}</div>
      </div>
    {:else}
      <div class="dv-empty-state">
        <svg viewBox="0 0 24 24" width="32" height="32"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
        <span>Write a query and press Run</span>
      </div>
    {/if}
  {:else if activeTab === 'aggregation'}
    <div class="dv-query-area">
      <QueryEditor bind:value={aggPipeline} label="Aggregation Pipeline" placeholder={'[{"$match": {}}]'} onexecute={handleAggExecute} />
    </div>
    {#if queryResult}
      <div class="dv-query-result">
        <div class="dv-query-meta">
          <span>{queryResult.length} results</span>
          <span class="dv-meta-sep">·</span>
          <span>{queryDuration}ms</span>
        </div>
        <div class="dv-query-body">{@html highlightJSON(JSON.stringify(queryResult, null, 2))}</div>
      </div>
    {:else}
      <div class="dv-empty-state">
        <svg viewBox="0 0 24 24" width="32" height="32"><polyline points="22 12 18 12 15 21 9 3 6 12 2 12"/></svg>
        <span>Write a pipeline and press Run</span>
      </div>
    {/if}
  {/if}
</div>

<style>
  .dv-wrap {
    display: flex;
    flex-direction: column;
    flex: 1;
    overflow: hidden;
  }

  /* ── Tabs ── */
  .dv-tabs {
    height: 34px;
    flex-shrink: 0;
    background: var(--n2);
    border-bottom: 1px solid var(--b1);
    padding: 0 12px;
    display: flex;
    align-items: center;
    gap: 2px;
  }
  .dv-tab {
    font-size: 12px;
    color: var(--t3);
    cursor: default;
    font-family: var(--mono);
    padding: 4px 10px 6px;
    border: none;
    background: transparent;
    border-bottom: 1.5px solid transparent;
    transition: color 0.1s;
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .dv-tab svg {
    stroke: currentColor;
    fill: none;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0.6;
  }
  .dv-tab.on {
    color: var(--t1);
    border-bottom-color: var(--acc);
  }
  .dv-tab.on svg { opacity: 1; }
  .dv-tab:hover:not(.on) { color: var(--t2); }
  .dv-tab-spacer { flex: 1; }
  .dv-meta {
    font-size: 10.5px;
    color: var(--t4);
    font-family: var(--mono);
  }
  .dv-meta-sep {
    color: var(--t4);
    font-size: 10px;
    margin: 0 2px;
  }

  /* ── Filter bar ── */
  .dv-filter-bar {
    flex-shrink: 0;
    background: var(--n2);
    border-bottom: 1px solid var(--b1);
    padding: 6px 10px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .dv-filter-input {
    flex: 1;
    min-height: 26px;
    max-height: 200px;
    background: rgba(255,255,255,0.04);
    border: 1px solid var(--b1);
    border-radius: 5px;
    padding: 5px 8px;
    font-family: var(--mono);
    font-size: 12.5px;
    line-height: 1.4;
    color: var(--t1);
    outline: none;
    transition: border-color 0.15s;
    resize: none;
    overflow: hidden;
  }
  .dv-filter-input:focus {
    border-color: var(--acc);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--acc) 10%, transparent);
    overflow-y: auto;
  }
  .dv-filter-input::placeholder {
    color: var(--t4);
  }
  .dv-bar-actions {
    display: flex;
    align-items: center;
    gap: 5px;
    flex-shrink: 0;
  }
  .dv-bar-btn {
    height: 26px;
    padding: 0 10px;
    border-radius: 5px;
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t2);
    font-size: 11.5px;
    font-family: var(--ui);
    font-weight: 500;
    cursor: default;
    flex-shrink: 0;
    transition: border-color 0.1s, color 0.1s, background 0.1s;
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .dv-bar-btn:hover {
    border-color: var(--b2);
    color: var(--t1);
  }
  .dv-find-btn {
    background: var(--acc);
    border-color: var(--acc);
    color: #fff;
  }
  .dv-find-btn:hover {
    opacity: 0.85;
  }
  .dv-reset-btn {
    color: var(--t3);
  }
  .dv-reset-btn:hover {
    color: var(--t1);
  }
  .dv-options-btn {
    color: var(--t3);
  }
  .dv-options-btn.active {
    color: var(--acc);
    border-color: var(--acc);
  }
  .dv-options-btn svg {
    stroke: currentColor;
    fill: none;
  }
  .dv-options-row {
    flex-shrink: 0;
    background: var(--n2);
    border-bottom: 1px solid var(--b1);
    padding: 6px 10px;
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .dv-option-label {
    font-size: 11px;
    font-weight: 500;
    color: var(--t3);
    font-family: var(--ui);
    flex-shrink: 0;
  }
  .dv-option-input {
    flex: 1;
    height: 26px;
    background: rgba(255,255,255,0.04);
    border: 1px solid var(--b1);
    border-radius: 5px;
    padding: 0 8px;
    font-family: var(--mono);
    font-size: 12.5px;
    color: var(--t1);
    outline: none;
    transition: border-color 0.15s;
  }
  .dv-option-input:focus {
    border-color: var(--acc);
    box-shadow: 0 0 0 2px color-mix(in srgb, var(--acc) 10%, transparent);
  }
  .dv-option-input::placeholder { color: var(--t4); }
  .dv-toolbar-btn {
    height: 28px;
    padding: 0 10px;
    border-radius: 5px;
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t2);
    font-size: 11.5px;
    font-family: var(--ui);
    font-weight: 500;
    cursor: default;
    flex-shrink: 0;
    transition: border-color 0.1s, color 0.1s, background 0.1s;
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .dv-toolbar-btn:hover {
    border-color: var(--b2);
    color: var(--t1);
  }
  .dv-toolbar-btn svg {
    stroke: currentColor;
    fill: none;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  /* Search bar */
  .dv-search-bar {
    height: 34px;
    flex-shrink: 0;
    background: var(--n2);
    border-bottom: 1px solid var(--b1);
    padding: 0 10px;
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--t3);
  }
  .dv-search-input {
    flex: 1;
    height: 24px;
    background: rgba(255,255,255,0.04);
    border: 1px solid var(--b1);
    border-radius: 4px;
    padding: 0 8px;
    font-family: var(--mono);
    font-size: 12px;
    color: var(--t1);
    outline: none;
  }
  .dv-search-input:focus {
    border-color: var(--acc);
  }
  .dv-search-count {
    font-size: 10.5px;
    color: var(--t3);
    font-family: var(--mono);
    white-space: nowrap;
  }
  .dv-search-nav {
    width: 22px;
    height: 22px;
    border-radius: 4px;
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t3);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: default;
    flex-shrink: 0;
  }
  .dv-search-nav:hover:not(:disabled) { color: var(--t1); border-color: var(--b2); }
  .dv-search-nav:disabled { opacity: 0.3; }
  :global(.dv-search-match) {
    background: rgba(250, 200, 50, 0.3);
    border-radius: 2px;
  }
  :global(.dv-active-match) {
    background: rgba(250, 200, 50, 0.6);
    outline: 1px solid rgba(250, 200, 50, 0.8);
  }

  .dv-add-wrap {
    position: relative;
    margin-left: 10px;
  }
  .dv-add-menu {
    position: absolute;
    top: calc(100% + 4px);
    right: 0;
    background: var(--modal-bg, var(--n));
    border: 1px solid var(--b1);
    border-radius: 8px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    z-index: 500;
    min-width: 200px;
    padding: 4px 0;
    animation: fadeIn 0.1s ease;
  }
  @keyframes fadeIn { from { opacity: 0; } to { opacity: 1; } }
  .dv-add-menu-item {
    width: 100%;
    padding: 8px 14px;
    font-size: 12.5px;
    color: var(--t2);
    font-family: var(--ui);
    cursor: default;
    background: transparent;
    border: none;
    text-align: left;
    display: flex;
    align-items: center;
    gap: 8px;
    transition: background 0.08s, color 0.08s;
  }
  .dv-add-menu-item:hover {
    background: var(--c);
    color: var(--t1);
  }
  .dv-add-menu-item svg {
    stroke: var(--t3);
    flex-shrink: 0;
  }
  .dv-add-menu-item:hover svg {
    stroke: var(--t1);
  }

  /* ── Document list ── */
  .dv-list {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    background: transparent;
  }
  .dv-list::-webkit-scrollbar { width: 4px; }
  .dv-list::-webkit-scrollbar-thumb { background: var(--b1); border-radius: 2px; }

  .dv-empty-state {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    padding: 48px 12px;
    color: var(--t4);
    font-size: 12px;
    font-family: var(--mono);
  }
  .dv-empty-state svg {
    stroke: var(--t4);
    fill: none;
    stroke-width: 1;
    stroke-linecap: round;
    stroke-linejoin: round;
    opacity: 0.5;
  }
  .dv-dots::after {
    content: '';
    animation: dvdots 1.4s steps(4, end) infinite;
  }
  @keyframes dvdots {
    0%   { content: ''; }
    25%  { content: '.'; }
    50%  { content: '..'; }
    75%  { content: '...'; }
  }

  .dv-doc {
    border-bottom: 1px solid var(--b1);
    padding: 0;
    transition: background 0.08s;
    border-left: 2px solid transparent;
  }
  .dv-doc.expanded {
    background: color-mix(in srgb, var(--acc) 5%, transparent);
    border-left-color: var(--acc);
  }
  .dv-doc-header {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    cursor: default;
  }
  .dv-doc-header:hover {
    background: color-mix(in srgb, var(--acc) 3%, transparent);
  }
  .dv-doc-chevron {
    color: var(--t4);
    display: flex;
    align-items: center;
    transition: transform 0.15s;
    flex-shrink: 0;
  }
  .dv-doc-chevron svg {
    stroke: currentColor;
    fill: none;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .dv-doc-chevron.open {
    transform: rotate(90deg);
    color: var(--acc);
  }
  .dv-doc-idx {
    font-size: 10px;
    color: var(--t4);
    font-family: var(--mono);
    min-width: 20px;
    flex-shrink: 0;
  }
  .dv-doc-id {
    font-size: 11px;
    color: var(--acc);
    font-family: var(--mono);
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    opacity: 0.8;
  }
  .dv-doc-actions {
    margin-left: auto;
    display: flex;
    gap: 2px;
    opacity: 0;
    transition: opacity 0.1s;
  }
  .dv-doc-header:hover .dv-doc-actions {
    opacity: 1;
  }
  .dv-doc-btn {
    width: 24px;
    height: 24px;
    border: none;
    background: transparent;
    color: var(--t3);
    cursor: default;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 5px;
    transition: color 0.1s, background 0.1s;
  }
  .dv-doc-btn svg {
    stroke: currentColor;
    fill: none;
    stroke-width: 1.8;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .dv-doc-btn:hover {
    color: var(--t1);
    background: var(--b1);
  }
  .dv-doc-btn.danger:hover {
    color: var(--err);
    background: color-mix(in srgb, var(--err) 10%, transparent);
  }
  .dv-doc-preview {
    font-size: 11px;
    color: var(--t3);
    font-family: var(--mono);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    padding-left: 30px;
    padding-right: 12px;
    padding-bottom: 6px;
    -webkit-user-select: text;
    user-select: text;
  }
  .dv-doc-body {
    font-family: var(--mono);
    font-size: 11.5px;
    line-height: 1.65;
    color: var(--t1);
    white-space: pre-wrap;
    word-break: break-word;
    padding: 4px 12px 10px 30px;
    -webkit-user-select: text;
    user-select: text;
    cursor: text;
    max-height: 300px;
    overflow-y: auto;
  }
  .dv-doc-body::-webkit-scrollbar { width: 4px; }
  .dv-doc-body::-webkit-scrollbar-thumb { background: var(--b1); border-radius: 2px; }

  /* Inline edit */
  .dv-doc-edit-wrap {
    display: flex;
    flex-direction: column;
    padding: 4px 12px 10px 12px;
    border-left: 2px solid var(--acc);
    margin-left: 16px;
  }
  .dv-edit-editor {
    display: flex;
    max-height: 300px;
    overflow-y: auto;
    border-radius: 5px;
    background: rgba(255,255,255,0.02);
    border: 1px solid color-mix(in srgb, var(--acc) 30%, transparent);
  }
  .dv-edit-editor::-webkit-scrollbar { width: 4px; }
  .dv-edit-editor::-webkit-scrollbar-thumb { background: var(--b1); border-radius: 2px; }

  .dv-edit-gutter {
    display: flex;
    flex-direction: column;
    padding: 8px 0;
    flex-shrink: 0;
    background: rgba(255,255,255,0.02);
    border-right: 1px solid var(--b1);
    user-select: none;
  }
  .dv-edit-ln {
    display: block;
    width: 32px;
    text-align: right;
    padding-right: 8px;
    color: var(--t4);
    font-family: var(--mono);
    font-size: 11.5px;
    line-height: 19px;
    height: 19px;
    opacity: 0.5;
    box-sizing: border-box;
  }
  .dv-ln-err {
    background: rgba(240, 68, 68, 0.15);
    color: var(--err);
    opacity: 1;
  }

  .dv-edit-ta {
    flex: 1;
    padding: 8px 12px;
    font-family: var(--mono);
    font-size: 11.5px;
    line-height: 19px;
    color: var(--t1);
    background: transparent;
    border: none;
    outline: none;
    resize: none;
    white-space: pre;
    overflow: hidden;
    box-sizing: border-box;
    tab-size: 2;
  }

  .dv-edit-error {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 6px;
    padding: 5px 8px;
    border-radius: 4px;
    background: rgba(240, 68, 68, 0.08);
    border: 1px solid rgba(240, 68, 68, 0.2);
    font-family: var(--mono);
    font-size: 10.5px;
    color: var(--err);
  }
  .dv-edit-error svg {
    stroke: var(--err);
    fill: none;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
    flex-shrink: 0;
  }

  .dv-doc-edit-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
    margin-top: 8px;
  }
  .dv-edit-cancel {
    height: 26px;
    padding: 0 12px;
    border-radius: 5px;
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t2);
    font-size: 11px;
    font-family: var(--ui);
    cursor: default;
    transition: border-color 0.1s, color 0.1s;
  }
  .dv-edit-cancel:hover {
    border-color: var(--b2);
    color: var(--t1);
  }
  .dv-edit-save {
    height: 26px;
    padding: 0 14px;
    border-radius: 5px;
    border: none;
    background: var(--acc);
    color: #fff;
    font-size: 11px;
    font-weight: 600;
    font-family: var(--ui);
    cursor: default;
    transition: opacity 0.1s;
  }
  .dv-edit-save:hover:not(:disabled) {
    opacity: 0.85;
  }
  .dv-edit-save:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }

  /* ── Footer ── */
  .dv-page-btn {
    width: 24px;
    height: 22px;
    border-radius: 4px;
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t2);
    cursor: default;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: border-color 0.1s, color 0.1s;
  }
  .dv-page-btn svg {
    stroke: currentColor;
    fill: none;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .dv-page-btn:hover:not(:disabled) {
    border-color: var(--b2);
    color: var(--t1);
  }
  .dv-page-btn:disabled {
    opacity: 0.25;
    cursor: default;
  }
  .dv-page-info {
    font-size: 10.5px;
    color: var(--t3);
    font-family: var(--mono);
    min-width: 40px;
    text-align: center;
  }
  .dv-limit-sel {
    height: 26px;
    background: rgba(255,255,255,0.04);
    border: 1px solid var(--b1);
    border-radius: 5px;
    padding: 0 6px;
    font-size: 11px;
    color: var(--t2);
    font-family: var(--mono);
    outline: none;
    cursor: default;
    margin-left: auto;
  }
  .dv-limit-sel option {
    background: var(--n);
    color: var(--t1);
  }

  /* ── Query area ── */
  .dv-query-area {
    display: flex;
    flex-direction: column;
    min-height: 120px;
    max-height: 40%;
    border-bottom: 1px solid var(--b1);
  }
  .dv-query-result {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    min-height: 0;
  }
  .dv-query-meta {
    height: 28px;
    flex-shrink: 0;
    background: var(--n2);
    border-bottom: 1px solid var(--b1);
    padding: 0 12px;
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 11px;
    color: var(--t3);
    font-family: var(--mono);
  }
  .dv-query-body {
    flex: 1;
    min-height: 0;
    background: transparent;
    padding: 12px 14px;
    overflow-y: auto;
    overflow-x: hidden;
    font-family: var(--mono);
    font-size: 11.5px;
    line-height: 1.65;
    white-space: pre-wrap;
    word-break: break-word;
    color: var(--t1);
    -webkit-user-select: text;
    user-select: text;
    cursor: text;
  }
  .dv-query-body::-webkit-scrollbar { width: 4px; }
  .dv-query-body::-webkit-scrollbar-thumb { background: var(--b1); border-radius: 2px; }

  /* ── JSON syntax highlighting ── */
  .dv-doc-body :global(.key)  { color: #c4a7e7; }
  .dv-doc-body :global(.str)  { color: #9ccfd8; }
  .dv-doc-body :global(.num)  { color: #f6c177; }
  .dv-doc-body :global(.boo)  { color: #eb6f92; }
  .dv-doc-body :global(.pu)   { color: var(--t4); }
  .dv-query-body :global(.key)  { color: #c4a7e7; }
  .dv-query-body :global(.str)  { color: #9ccfd8; }
  .dv-query-body :global(.num)  { color: #f6c177; }
  .dv-query-body :global(.boo)  { color: #eb6f92; }
  .dv-query-body :global(.pu)   { color: var(--t4); }

  /* ── Modals ── */
  .dv-modal-overlay {
    position: fixed;
    top: 0; left: 0; right: 0; bottom: 0;
    background: rgba(0,0,0,0.45);
    backdrop-filter: blur(8px);
    z-index: 1000;
    display: flex;
    align-items: center;
    justify-content: center;
    animation: dvFadeIn 0.12s ease;
  }
  @keyframes dvFadeIn {
    from { opacity: 0; }
    to { opacity: 1; }
  }
  .dv-modal {
    background: var(--modal-bg, var(--n));
    border: 1px solid var(--b1);
    border-radius: var(--radius-xl);
    box-shadow: 0 24px 48px rgba(0,0,0,0.45);
    width: 520px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    animation: dvSlideIn 0.15s ease;
  }
  @keyframes dvSlideIn {
    from { opacity: 0; transform: translateY(-8px) scale(0.98); }
    to { opacity: 1; transform: translateY(0) scale(1); }
  }
  .dv-modal-hdr {
    display: flex;
    align-items: center;
    padding: 14px 18px;
    border-bottom: 1px solid var(--b1);
    background: var(--n2);
    font-size: 14px;
    font-weight: 600;
    color: var(--t1);
    font-family: var(--ui);
  }
  .dv-modal-close {
    margin-left: auto;
    width: 28px;
    height: 28px;
    border-radius: 6px;
    border: 1px solid var(--b1);
    background: transparent;
    cursor: default;
    color: var(--t3);
    font-size: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: background 0.1s, color 0.1s;
  }
  .dv-modal-close:hover {
    background: var(--c);
    color: var(--t1);
  }
  .dv-modal-field {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin-bottom: 10px;
  }
  .dv-modal-label {
    font-size: 11px;
    font-weight: 600;
    color: var(--t2);
    font-family: var(--ui);
    text-transform: uppercase;
    letter-spacing: 0.05em;
    padding: 0 18px;
  }
  .dv-modal-lg {
    width: 680px;
  }

  /* Insert Document modal (Compass-style) */
  .dv-insert-modal {
    width: 720px;
    max-height: 80vh;
    display: flex;
    flex-direction: column;
  }
  .dv-insert-header {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    padding: 24px 24px 16px;
  }
  .dv-insert-title {
    font-size: 18px;
    font-weight: 700;
    color: var(--t1);
    font-family: var(--ui);
  }
  .dv-insert-subtitle {
    font-size: 12px;
    color: var(--t3);
    font-family: var(--ui);
    margin-top: 4px;
  }
  .dv-insert-toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 0 24px 8px;
  }
  .dv-insert-tool-btn {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 4px 10px;
    border-radius: 6px;
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t3);
    font-size: 11px;
    font-family: var(--ui);
    cursor: default;
    transition: all 0.1s;
  }
  .dv-insert-tool-btn:hover {
    background: var(--c);
    color: var(--t1);
    border-color: var(--b2);
  }
  .dv-insert-doc-count {
    margin-left: auto;
    font-size: 11px;
    color: var(--t3);
    font-family: var(--ui);
    background: color-mix(in srgb, var(--acc) 10%, transparent);
    color: var(--acc);
    padding: 2px 8px;
    border-radius: 4px;
    font-weight: 500;
  }
  .dv-insert-editor-area {
    padding: 0 24px;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .dv-insert-editor-container {
    position: relative;
    min-height: 320px;
    max-height: 50vh;
    overflow-y: auto;
    border: 1px solid var(--b1);
    border-radius: 8px;
  }
  .dv-insert-editor {
    position: relative;
    z-index: 1;
    min-height: 320px;
    border: none;
    border-radius: 0;
  }
  .dv-insert-editor .dv-edit-ta {
    min-height: 300px;
    color: transparent;
    caret-color: var(--t1);
  }
  .dv-insert-highlight {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    padding: 8px 12px 8px 48px;
    font-family: var(--mono);
    font-size: 12px;
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
    pointer-events: none;
    z-index: 0;
  }
  .dv-insert-footer {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 16px 24px 24px;
  }
  .dv-insert-hint {
    margin-right: auto;
    font-size: 11px;
    color: var(--t4);
    font-family: var(--ui);
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .dv-insert-hint kbd {
    background: var(--b1);
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 10px;
    font-family: var(--mono);
  }
  .dv-modal-editor-wrap {
    padding: 0 18px;
    margin-bottom: 10px;
  }
  .dv-modal-editor-wrap:first-of-type {
    margin-top: 4px;
  }
  .dv-modal-editor {
    max-height: 350px;
  }
  .dv-modal-editor-sm {
    max-height: 150px;
  }
  .dv-modal-actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    padding: 14px 18px;
    border-top: 1px solid var(--b1);
  }
  .dv-modal-cancel {
    height: 30px;
    padding: 0 14px;
    border-radius: 6px;
    border: 1px solid var(--b1);
    background: transparent;
    color: var(--t2);
    font-size: 11.5px;
    font-family: var(--ui);
    cursor: default;
    transition: border-color 0.12s, color 0.12s;
  }
  .dv-modal-cancel:hover {
    border-color: var(--b2);
    color: var(--t1);
  }
  .dv-modal-primary {
    height: 30px;
    padding: 0 16px;
    border-radius: 6px;
    border: none;
    background: var(--acc);
    color: #fff;
    font-size: 11.5px;
    font-weight: 600;
    font-family: var(--ui);
    cursor: default;
    transition: opacity 0.12s;
  }
  .dv-modal-primary:hover:not(:disabled) {
    opacity: 0.85;
  }
  .dv-modal-primary:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .dv-modal-primary.danger {
    background: var(--err);
  }
</style>
