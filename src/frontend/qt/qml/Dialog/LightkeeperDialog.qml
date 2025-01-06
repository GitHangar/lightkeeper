import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import ".."
import "../Text"
import "../Misc"


Dialog {
    id: root
    modal: true
    opacity: visible ? 1.0 : 0.0
    title: ""
    // Use `anchors.centerIn: undefined` to avoid automatic centering.
    anchors.centerIn: parent

    property int borderRadius: 6
    property color headerBackground: Theme.titleBarColor

    background: Rectangle {
        width: root.width
        // To hide the top border, add border.width to the height so it goes under header.
        height: root.height - customHeader.height + border.width
        anchors.bottom: parent.bottom
        color: Theme.backgroundColor
        border.color: Theme.borderColor
        border.width: 1
    }

    Overlay.modal: Rectangle {
        color: "#60000000"
    }

    header: Rectangle {
        id: customHeader
        width: root.width
        height: 30
        radius: root.borderRadius
        color: root.headerBackground
        border.color: Theme.borderColor
        border.width: 1

        // Cover the border and rounding on bottom corners.
        Rectangle {
            anchors.bottom: parent.bottom
            width: root.width - border.width
            height: root.borderRadius
            color: root.headerBackground
        }

        NormalText {
            anchors.centerIn: parent
            text: root.title
        }
    }

    Behavior on opacity {
        NumberAnimation {
            duration: Theme.animationDurationFast
        }
    }
}