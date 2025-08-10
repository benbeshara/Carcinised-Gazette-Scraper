function updatePolygons(geojsonData) {
    currentPolygons.forEach(function(polygon) {
        polygon.remove();
    });
    currentPolygons = [];

    L.geoJSON(geojsonData, {
        pointToLayer: (feature, latlng) => {
            return new L.Circle(latlng, {radius: 500});
        },
        style: function(feature) {
            let start = Date.parse(feature.properties.start);
            let end = Date.parse(feature.properties.end);
            let beyond = end + 1000 * 60 * 60 * 24;
            let date = Date.now()

            if(date > beyond){
                return {
                    color: '#4444ff',
                    weight: 2,
                    opacity: 0.25
                }
            } else if(start <= date){
                return {
                    color: '#ff0000',
                    weight: 2,
                    opacity: 0.65
                }
            }
            return {
                color: '#ffec16',
                weight: 2,
                opacity: 0.25
            };
        },
        onEachFeature: function(feature, layer) {
            const popupContent = `
                                    <div class='custom-popup'>
                                        <p><strong>${feature.properties.title}</strong></p>
                                        <p><strong>In effect from:</strong> ${feature.properties.start} until ${feature.properties.end}</p>
                                        <p><a href='${feature.properties.uri}' target='_blank'>View Details</a></p>
                                        ${feature.properties.img_uri ?
                                        `<img alt='${feature.properties.title}' src='${feature.properties.img_uri}' style='max-width: 200px;'>`
                                            : ''}
                                        </div>
                                    `;
            layer.bindPopup(popupContent);
            currentPolygons.push(layer);
        }
    }).addTo(map);
}
