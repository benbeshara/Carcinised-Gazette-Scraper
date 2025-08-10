let map = L.map('map', {maxBounds: [[-39.2, 140.7],[-33.9,149.0]], minZoom: 7}).setView([-37.81705, 144.96326], 10);
L.tileLayer('https://{s}.basemaps.cartocdn.com/light_all/{z}/{x}/{y}{r}.png', {
    attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors &copy; <a href="https://carto.com/attributions">CARTO</a>',
    subdomains: 'abcd',
    maxZoom: 19
}).addTo(map);

setTimeout(function () {
    map.invalidateSize(true);
}, 100);
let currentPolygons = [];
