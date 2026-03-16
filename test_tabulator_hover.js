document.addEventListener('mouseover', function(e) {
  if(e.target && e.target.classList.contains('tabulator-menu-item-submenu')) {
    // If Tabulator uses click to open submenus, simulate a click when mouse enters
    // This allows hover to expand the menu without needing Svelte changes
    if(!e.target.dataset.hoverOpened) {
        e.target.dataset.hoverOpened = "true";
        e.target.click();
    }
  }
}, true);
