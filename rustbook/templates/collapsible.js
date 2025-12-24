// Collapsible chapters functionality

(function() {
    'use strict';

    // Find all expandable chapters
    var expandables = document.querySelectorAll('.chapter.expandable');

    expandables.forEach(function(chapter) {
        var toggle = chapter.querySelector('a, .chapter-title');
        if (!toggle) return;

        // Check if current page is within this chapter
        var articles = chapter.querySelector('.articles');
        if (articles) {
            var activeChild = articles.querySelector('.active');
            if (activeChild || chapter.classList.contains('active')) {
                chapter.classList.add('expanded');
            }
        }

        // Handle click on chapter title (only for chapters without links)
        var link = chapter.querySelector('a');
        var titleSpan = chapter.querySelector('.chapter-title');

        if (titleSpan) {
            titleSpan.addEventListener('click', function(e) {
                e.preventDefault();
                chapter.classList.toggle('expanded');
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
            }
            parent = parent.parentElement;
        }
    }
})();
