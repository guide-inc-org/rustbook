// Collapsible chapters functionality with state persistence

(function() {
    'use strict';

    var STORAGE_KEY = 'rustbook-expanded-chapters';

    // Load expanded state from localStorage
    function loadExpandedState() {
        try {
            var saved = localStorage.getItem(STORAGE_KEY);
            return saved ? JSON.parse(saved) : {};
        } catch (e) {
            return {};
        }
    }

    // Save expanded state to localStorage
    function saveExpandedState(state) {
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
        } catch (e) {
            // Ignore storage errors
        }
    }

    // Get unique identifier for a chapter based on its link or text
    function getChapterId(chapter) {
        var link = chapter.querySelector('a');
        if (link && link.getAttribute('href')) {
            return link.getAttribute('href');
        }
        var title = chapter.querySelector('.chapter-title');
        if (title) {
            return 'title:' + title.textContent.trim();
        }
        return null;
    }

    var expandedState = loadExpandedState();

    // Find all expandable chapters
    var expandables = document.querySelectorAll('.chapter.expandable');

    expandables.forEach(function(chapter) {
        var toggle = chapter.querySelector('a, .chapter-title');
        if (!toggle) return;

        var chapterId = getChapterId(chapter);
        var articles = chapter.querySelector('.articles');

        // Determine initial expanded state
        var shouldExpand = false;

        // Check if current page is within this chapter (always expand parent of active)
        if (articles) {
            var activeChild = articles.querySelector('.active');
            if (activeChild || chapter.classList.contains('active')) {
                shouldExpand = true;
                // Also save this state
                if (chapterId) {
                    expandedState[chapterId] = true;
                }
            }
        }

        // Check saved state (if not already determined by active status)
        if (!shouldExpand && chapterId && expandedState[chapterId]) {
            shouldExpand = true;
        }

        if (shouldExpand) {
            chapter.classList.add('expanded');
        }

        // Handle click on chapter title (only for chapters without links)
        var link = chapter.querySelector('a');
        var titleSpan = chapter.querySelector('.chapter-title');

        if (titleSpan) {
            titleSpan.addEventListener('click', function(e) {
                e.preventDefault();
                chapter.classList.toggle('expanded');

                // Save state
                if (chapterId) {
                    expandedState[chapterId] = chapter.classList.contains('expanded');
                    saveExpandedState(expandedState);
                }
            });
        }

        // For linked chapters, add a toggle button
        if (link && articles) {
            var toggleBtn = document.createElement('span');
            toggleBtn.className = 'toggle-btn';
            toggleBtn.innerHTML = '';
            toggleBtn.style.cssText = 'cursor: pointer; padding: 5px; margin-left: -20px; position: absolute;';

            link.style.position = 'relative';

            toggleBtn.addEventListener('click', function(e) {
                e.preventDefault();
                e.stopPropagation();
                chapter.classList.toggle('expanded');

                // Save state
                if (chapterId) {
                    expandedState[chapterId] = chapter.classList.contains('expanded');
                    saveExpandedState(expandedState);
                }
            });
        }

        // Also allow clicking the arrow (▸) to toggle
        if (link) {
            link.addEventListener('click', function(e) {
                // Check if click is on the arrow area (left side)
                var rect = link.getBoundingClientRect();
                var clickX = e.clientX - rect.left;

                // If clicked on the left 25px (arrow area), toggle instead of navigate
                if (clickX < 25 && articles) {
                    e.preventDefault();
                    e.stopPropagation();
                    chapter.classList.toggle('expanded');

                    // Save state
                    if (chapterId) {
                        expandedState[chapterId] = chapter.classList.contains('expanded');
                        saveExpandedState(expandedState);
                    }
                }
            });
        }
    });

    // Expand parent chapters of active item
    var active = document.querySelector('.chapter.active');
    if (active) {
        var parent = active.parentElement;
        while (parent) {
            if (parent.classList && parent.classList.contains('chapter')) {
                parent.classList.add('expanded');
                // Save parent expanded state
                var parentId = getChapterId(parent);
                if (parentId) {
                    expandedState[parentId] = true;
                }
            }
            parent = parent.parentElement;
        }
        saveExpandedState(expandedState);
    }
})();
