import QtQuick 2.15
import Qt.labs.qmlmodels 1.0

Item {
    id: root
    property bool firstItem: false
    property bool selected: false
    implicitHeight: 40

    signal clicked()

    // Stylish rounded cell for first item.
    Rectangle {
        id: rounded
        anchors.fill: parent
        radius: parent.firstItem ? 9 : 0
        color: getBackgroundColor(root.selected)

        MouseArea {
            anchors.fill: parent
            onClicked: root.clicked()
        }
    }

    Rectangle {
        color: getBackgroundColor(root.selected)
        width: rounded.radius
        anchors.top: rounded.top
        anchors.bottom: rounded.bottom
        anchors.right: rounded.right
    }

    function getBackgroundColor(selected) {
        if (selected === true) {
            return Theme.color_highlight()
        }
        else if (model.row % 2 == 0) {
            return palette.alternateBase
            return Theme.color_table_background()
        }
        else {
            return palette.base
            return Theme.color_background()
        }
    }
}