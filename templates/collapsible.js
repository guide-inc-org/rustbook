// Collapsible chapters functionality with localStorage persistence

(function() {
    'use strict';

    var STORAGE_KEY = 'guidebook-expanded-chapters';
    var sidebar = document.querySelector('.book-summary');
    if (!sidebar) return;

    // Get stored expanded state
    function getExpandedState() {
        try {
            var stored = localStorage.getItem(STORAGE_KEY);
            return stored ? JSON.parse(stored) : {};
        } catch (e) {
            return {};
        }
    }

    // Save expanded state
    function saveExpandedState(state) {
        try {
            localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
        } catch (e) {}
    }

    // Get unique identifier for a chapter (based on its link href or title)
    function getChapterId(chapter) {
        var link = chapter.querySelector(':scope > a');
        if (link) {
            return link.getAttribute('href');
        }
        var title = chapter.querySelector(':scope > .chapter-title');
        if (title) {
            return 'title:' + title.textContent.trim();
        }
        return null;
    }

    // Restore expanded state from localStorage
    function restoreExpandedState() {
        var state = getExpandedState();
        var chapters = sidebar.querySelectorAll('.chapter.expandable');

        chapters.forEach(function(chapter) {
            var id = getChapterId(chapter);
            if (id && state[id] !== undefined) {
                if (state[id]) {
                    chapter.classList.add('expanded');
                } else {
                    // Only close if not in the active path
                    if (!chapter.classList.contains('active') &&
                        !chapter.querySelector('.chapter.active')) {
                        chapter.classList.remove('expanded');
                    }
                }
            }
        });
    }

    // Save current expanded state to localStorage
    function saveCurrentState() {
        var state = {};
        var chapters = sidebar.querySelectorAll('.chapter.expandable');

        chapters.forEach(function(chapter) {
            var id = getChapterId(chapter);
            if (id) {
                state[id] = chapter.classList.contains('expanded');
            }
        });

        saveExpandedState(state);
    }

    // Restore state on page load
    restoreExpandedState();

    // Use event delegation so it works after SPA navigation
    sidebar.addEventListener('click', function(e) {
        var link = e.target.closest('a');
        var titleSpan = e.target.closest('.chapter-title');
        var chapter = e.target.closest('.chapter.expandable');

        if (!chapter) return;

        var articles = chapter.querySelector('.articles');
        if (!articles) return;

        // If clicked on chapter-title (no link), toggle expand
        if (titleSpan && !link) {
            e.preventDefault();
            e.stopImmediatePropagation();
            chapter.classList.toggle('expanded');
            saveCurrentState();
            return;
        }

        // If clicked on link, check if it's on the arrow area (left 25px)
        if (link) {
            var rect = link.getBoundingClientRect();
            var clickX = e.clientX - rect.left;

            if (clickX < 25) {
                e.preventDefault();
                e.stopImmediatePropagation();
                chapter.classList.toggle('expanded');
                saveCurrentState();
            }
        }
    });
})();
