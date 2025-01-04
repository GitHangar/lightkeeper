import QtQuick
import QtQuick.Controls

Rectangle {
    id: root
    default property alias contentItem: background.data
    property int border: 0
    property int borderBottom: border > 0 ? border : 0
    property int borderTop: border > 0 ? border : 0
    property int borderLeft: border > 0 ? border : 0
    property int borderRight: border > 0 ? border : 0
    property color borderColor: palette.base
    property alias backgroundColor: background.color

    color: borderColor

    Rectangle {
        id: background
        anchors.fill: parent
        anchors.topMargin: root.borderTop
        anchors.bottomMargin: root.borderBottom
        anchors.leftMargin: root.borderLeft
        anchors.rightMargin: root.borderRight
    }
}