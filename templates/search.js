// Search functionality for Guidebook

(function() {
    'use strict';

    var searchIndex = null;
    var searchInput = document.querySelector('.search-input');
    var searchResults = document.querySelector('.search-results');
    var searchWrapper = document.querySelector('.search-wrapper');

    if (!searchInput || !searchResults) return;

    // Load search index
    function loadSearchIndex() {
        if (searchIndex) return Promise.resolve(searchIndex);

        var baseUrl = document.querySelector('base');
        var indexUrl = baseUrl ? baseUrl.href + 'search_index.json' : 'search_index.json';

        return fetch(indexUrl)
            .then(function(response) {
                if (!response.ok) throw new Error('Search index not found');
                return response.json();
            })
            .then(function(data) {
                searchIndex = data;
                return data;
            })
            .catch(function(err) {
                console.error('Failed to load search index:', err);
                return [];
            });
    }

    // Simple search function
    function search(query) {
        if (!searchIndex || !query) return [];

        var lowerQuery = query.toLowerCase();
        var results = [];

        searchIndex.forEach(function(entry) {
            var titleMatch = entry.title.toLowerCase().indexOf(lowerQuery);
            var contentMatch = entry.content.toLowerCase().indexOf(lowerQuery);

            if (titleMatch !== -1 || contentMatch !== -1) {
                var score = 0;
                if (titleMatch !== -1) score += 10;
                if (contentMatch !== -1) score += 1;

                // Extract snippet around match
                var snippet = '';
                if (contentMatch !== -1) {
                    var start = Math.max(0, contentMatch - 50);
                    var end = Math.min(entry.content.length, contentMatch + query.length + 50);
                    snippet = (start > 0 ? '...' : '') +
                              entry.content.substring(start, end) +
                              (end < entry.content.length ? '...' : '');
                }

                results.push({
                    title: entry.title,
                    path: entry.path,
                    snippet: snippet,
                    score: score
                });
            }
        });

        // Sort by score (higher first)
        results.sort(function(a, b) {
            return b.score - a.score;
        });

        return results.slice(0, 10); // Limit to 10 results
    }

    // Render search results
    function renderResults(results, query) {
        if (results.length === 0) {
            searchResults.innerHTML = '<div class="search-no-results">No results found</div>';
            return;
        }

        var html = results.map(function(result) {
            var highlightedTitle = highlightMatch(result.title, query);
            var highlightedSnippet = result.snippet ? highlightMatch(result.snippet, query) : '';

            return '<a class="search-result-item" href="' + result.path + '">' +
                   '<div class="search-result-title">' + highlightedTitle + '</div>' +
                   (highlightedSnippet ? '<div class="search-result-snippet">' + highlightedSnippet + '</div>' : '') +
                   '</a>';
        }).join('');

        searchResults.innerHTML = html;
    }

    // Highlight matching text
    function highlightMatch(text, query) {
        if (!query) return escapeHtml(text);

        var escaped = escapeHtml(text);
        var regex = new RegExp('(' + escapeRegex(query) + ')', 'gi');
        return escaped.replace(regex, '<mark>$1</mark>');
    }

    function escapeHtml(text) {
        var div = document.createElement('div');
        div.textContent = text;
        return div.innerHTML;
    }

    function escapeRegex(str) {
        return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    }

    // Event handlers
    var debounceTimer = null;

    searchInput.addEventListener('input', function() {
        var query = this.value.trim();

        clearTimeout(debounceTimer);

        if (query.length < 1) {
            searchResults.classList.remove('visible');
            return;
        }

        debounceTimer = setTimeout(function() {
            loadSearchIndex().then(function() {
                var results = search(query);
                renderResults(results, query);
                searchResults.classList.add('visible');
            });
        }, 200);
    });

    searchInput.addEventListener('focus', function() {
        if (this.value.trim().length >= 1) {
            searchResults.classList.add('visible');
        }
    });

    // Close results when clicking outside
    document.addEventListener('click', function(e) {
        if (searchWrapper && !searchWrapper.contains(e.target)) {
            searchResults.classList.remove('visible');
        }
    });

    // Keyboard navigation
    searchInput.addEventListener('keydown', function(e) {
        if (e.key === 'Escape') {
            searchResults.classList.remove('visible');
            this.blur();
        }
    });

    // Preload search index on first focus
    searchInput.addEventListener('focus', function() {
        loadSearchIndex();
    }, { once: true });
})();
