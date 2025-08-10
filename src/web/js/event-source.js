const eventSource = new EventSource('/data');
eventSource.addEventListener('close', function(e) {
    const geojsonData = JSON.parse(e.data);
    updatePolygons(geojsonData);
    document.getElementById('notice').outerHTML = '';
    eventSource.close();
});
eventSource.addEventListener('circles', function(e) {
    const circleData = JSON.parse('[' + e.data + ']');
    updateCircles(circleData);
});
eventSource.addEventListener('list', function(e) {
    document.getElementById('list').outerHTML = e.data;
});
window.addEventListener('unload', function() {
    if (eventSource) {
        eventSource.close();
    }
});