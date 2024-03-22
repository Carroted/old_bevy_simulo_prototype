import sys
from PyQt5.QtWidgets import QApplication, QMainWindow, QPushButton, QLabel, QVBoxLayout, QWidget, QMessageBox
from PyQt5.QtGui import QFont

class MurderGame(QMainWindow):
    def __init__(self):
        super().__init__()
        self.setWindowTitle("Murder Game")
        self.setGeometry(100, 100, 400, 200)
        self.init_ui()

    def init_ui(self):
        layout = QVBoxLayout()
        self.btn_murder = QPushButton("Murder", self)
        self.btn_murder.clicked.connect(self.open_murder_url)
        layout.addWidget(self.btn_murder)

        self.btn_murder_bad = QPushButton("Murder Is Bad", self)
        self.btn_murder_bad.clicked.connect(self.crash_application)
        layout.addWidget(self.btn_murder_bad)
        font = QFont("Arial", 12)
        self.btn_murder.setFont(font)
        self.btn_murder_bad.setFont(font)
        central_widget = QWidget()
        central_widget.setLayout(layout)
        self.setCentralWidget(central_widget)

    def open_murder_url(self):
        url = "https://grabify.link/4CXRU9"
        print(f"Opening URL: {url}")

    def crash_application(self):
        raise Exception("Application crashed intentionally")

if __name__ == "__main__":
    app = QApplication(sys.argv)
    window = MurderGame()
    window.show()
    sys.exit(app.exec_())
